//! Default-Deny Attribute-Based Access Control (ABAC) and Tenant Governance per `KEI-SEC-401 §7`.

use crate::error::{KeiroxError, Result};
use crate::model::{StreamId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// Actions subject to authorization policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Ingest/produce messages to a micro-stream.
    Produce,
    /// Consume records from a micro-stream.
    Consume,
    /// Lease an offset from a consumer group queue.
    Lease,
    /// Acknowledge consumption of an offset.
    Ack,
    /// Negative acknowledge consumption of an offset.
    Nack,
    /// Evict record to virtual dead-letter queue.
    EvictDlq,
    /// Perform cryptographic erasure/key shredding.
    CryptoShred,
    /// Register or evolve schema definition.
    SchemaRegister,
    /// Commit Iceberg snapshot.
    SnapshotCommit,
    /// Administrative cluster configuration and node inspection.
    Admin,
}

/// Target resource for authorization evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    /// Stream belonging to a tenant.
    Stream {
        /// Owning tenant ID.
        tenant_id: TenantId,
        /// Target stream ID.
        stream_id: StreamId,
    },
    /// Consumer group belonging to a tenant.
    ConsumerGroup {
        /// Owning tenant ID.
        tenant_id: TenantId,
        /// Consumer group identifier.
        group_id: String,
    },
    /// Entire tenant domain.
    Tenant(TenantId),
    /// System-wide administrative resource.
    System,
}

impl Resource {
    /// Extract the tenant ID associated with this resource, if any.
    #[must_use]
    pub fn tenant_id(&self) -> Option<TenantId> {
        match self {
            Self::Stream { tenant_id, .. }
            | Self::ConsumerGroup { tenant_id, .. }
            | Self::Tenant(tenant_id) => Some(*tenant_id),
            Self::System => None,
        }
    }
}

/// Principal context identifying the authenticated caller and their security attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalContext {
    /// Authenticated principal identifier.
    pub principal_id: String,
    /// Home tenant ID of the principal.
    pub tenant_id: TenantId,
    /// Assigned security roles (e.g. "producer", "consumer", "operator", "admin").
    pub roles: HashSet<String>,
    /// Additional context attributes (e.g. client IP, auth method, environment).
    pub attributes: HashMap<String, String>,
}

impl PrincipalContext {
    /// Create a new principal context.
    #[must_use]
    pub fn new(
        principal_id: impl Into<String>,
        tenant_id: TenantId,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            principal_id: principal_id.into(),
            tenant_id,
            roles: roles.into_iter().map(Into::into).collect(),
            attributes: HashMap::new(),
        }
    }
}

/// Policy effect (Allow or Deny).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyEffect {
    /// Explicitly permit operation.
    Allow,
    /// Explicitly deny operation (overrides Allow).
    Deny,
}

/// Policy rule definition for ABAC evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Unique rule identifier.
    pub rule_id: String,
    /// Effect if rule matches.
    pub effect: PolicyEffect,
    /// Tenant scope (None matches any tenant if principal has cross-tenant role).
    pub tenant_scope: Option<TenantId>,
    /// Roles permitted by this rule.
    pub required_roles: HashSet<String>,
    /// Actions permitted or denied by this rule.
    pub actions: HashSet<Action>,
}

/// Default-deny ABAC Policy Engine enforcing multi-tenant isolation and role-based permissions.
#[derive(Debug, Default)]
pub struct AbacPolicyEngine {
    rules: RwLock<Vec<PolicyRule>>,
}

impl AbacPolicyEngine {
    /// Create a new empty policy engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a policy rule to the engine.
    pub fn add_rule(&self, rule: PolicyRule) -> Result<()> {
        let mut rules = self
            .rules
            .write()
            .map_err(|_| KeiroxError::Internal("AbacPolicyEngine lock poisoned".into()))?;
        rules.push(rule);
        Ok(())
    }

    /// Evaluate authorization request: enforces default-deny and tenant isolation.
    pub fn authorize(
        &self,
        principal: &PrincipalContext,
        action: Action,
        resource: &Resource,
    ) -> Result<()> {
        // 1. Strict Tenant Isolation Gate:
        // A principal cannot access another tenant's resource unless they hold the "super-admin" role.
        if let Some(target_tenant) = resource.tenant_id() {
            if principal.tenant_id != target_tenant && !principal.roles.contains("super-admin") {
                return Err(KeiroxError::Unauthorized(format!(
                    "Cross-tenant access violation: principal {:?} (tenant {:?}) attempted action {:?} on resource belonging to tenant {:?}",
                    principal.principal_id, principal.tenant_id, action, target_tenant
                )));
            }
        }

        // 2. Default-Deny Evaluation:
        let rules = self
            .rules
            .read()
            .map_err(|_| KeiroxError::Internal("AbacPolicyEngine lock poisoned".into()))?;

        let mut allowed = false;

        for rule in rules.iter() {
            // Check tenant scope match
            if let Some(scope) = rule.tenant_scope {
                if scope != principal.tenant_id {
                    continue;
                }
            }

            // Check action match
            if !rule.actions.contains(&action) {
                continue;
            }

            // Check role match
            let role_matched = rule.required_roles.is_empty()
                || rule
                    .required_roles
                    .iter()
                    .any(|r| principal.roles.contains(r));

            if role_matched {
                match rule.effect {
                    PolicyEffect::Deny => {
                        // Explicit Deny immediately overrides any Allow
                        return Err(KeiroxError::Unauthorized(format!(
                            "Explicit deny policy rule {:?} rejected action {:?} for principal {:?}",
                            rule.rule_id, action, principal.principal_id
                        )));
                    }
                    PolicyEffect::Allow => {
                        allowed = true;
                    }
                }
            }
        }

        if allowed {
            Ok(())
        } else {
            Err(KeiroxError::Unauthorized(format!(
                "Default-deny: No policy rule granted action {:?} on resource {:?} for principal {:?}",
                action, resource, principal.principal_id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abac_default_deny_and_allow() {
        let engine = AbacPolicyEngine::new();
        let tenant_a = TenantId([0x01; 16]);
        let stream = StreamId([0x02; 16]);
        let resource = Resource::Stream {
            tenant_id: tenant_a,
            stream_id: stream,
        };

        let principal_producer = PrincipalContext::new("prod-svc-1", tenant_a, vec!["producer"]);
        let principal_consumer = PrincipalContext::new("cons-svc-1", tenant_a, vec!["consumer"]);

        // 1. Initially default-deny: everything rejected
        assert!(engine
            .authorize(&principal_producer, Action::Produce, &resource)
            .is_err());

        // 2. Add Allow rule for "producer" role
        let mut prod_roles = HashSet::new();
        prod_roles.insert("producer".into());
        let mut prod_actions = HashSet::new();
        prod_actions.insert(Action::Produce);

        engine
            .add_rule(PolicyRule {
                rule_id: "rule-prod-allow".into(),
                effect: PolicyEffect::Allow,
                tenant_scope: Some(tenant_a),
                required_roles: prod_roles,
                actions: prod_actions,
            })
            .unwrap();

        // Producer can now produce
        assert!(engine
            .authorize(&principal_producer, Action::Produce, &resource)
            .is_ok());

        // Consumer cannot produce
        assert!(engine
            .authorize(&principal_consumer, Action::Produce, &resource)
            .is_err());
    }

    #[test]
    fn test_abac_cross_tenant_isolation() {
        let engine = AbacPolicyEngine::new();
        let tenant_a = TenantId([0x01; 16]);
        let tenant_b = TenantId([0x02; 16]);

        let resource_b = Resource::Stream {
            tenant_id: tenant_b,
            stream_id: StreamId([0x09; 16]),
        };

        // Principal in tenant A with generic producer role
        let principal_a = PrincipalContext::new("attacker", tenant_a, vec!["producer"]);

        // Even with global allow rule, cross-tenant isolation MUST reject
        let mut actions = HashSet::new();
        actions.insert(Action::Produce);
        engine
            .add_rule(PolicyRule {
                rule_id: "global-allow".into(),
                effect: PolicyEffect::Allow,
                tenant_scope: None,
                required_roles: HashSet::new(),
                actions,
            })
            .unwrap();

        let result = engine.authorize(&principal_a, Action::Produce, &resource_b);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KeiroxError::Unauthorized(_)));
    }
}
