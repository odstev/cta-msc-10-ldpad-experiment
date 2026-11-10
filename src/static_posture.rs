//! Layer 4: Static Posture Auditing
//! Audits Kubernetes-style resource configurations against a fixed
//! rule set derived from CIS Benchmark controls.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sResource {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub privileged: bool,
    pub run_as_root: bool,
    pub read_only_root_fs: bool,
    pub has_network_policy: bool,
    pub rbac_least_privilege: bool,
    pub image_tag_is_latest: bool,
    pub has_resource_limits: bool,
}

/// Rule contains a function pointer, which serde cannot serialize.
/// Keep only Debug + Clone on this struct.
#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub severity: f64,
    pub check: fn(&K8sResource) -> bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub resource: String,
    pub rule: String,
    pub passed: bool,
    pub severity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAuditReport {
    pub findings: Vec<Finding>,
    pub static_risk: f64,
    pub rules_evaluated: usize,
}

pub struct Auditor {
    rules: Vec<Rule>,
}

impl Auditor {
    pub fn new() -> Self {
        Self {
            rules: vec![
                Rule { name: "non_privileged".into(),         severity: 30.0, check: |r| !r.privileged },
                Rule { name: "non_root".into(),               severity: 20.0, check: |r| !r.run_as_root },
                Rule { name: "read_only_rootfs".into(),       severity: 15.0, check: |r| r.read_only_root_fs },
                Rule { name: "network_policy_present".into(), severity: 20.0, check: |r| r.has_network_policy },
                Rule { name: "rbac_least_privilege".into(),   severity: 15.0, check: |r| r.rbac_least_privilege },
                Rule { name: "image_not_latest".into(),       severity: 10.0, check: |r| !r.image_tag_is_latest },
                Rule { name: "resource_limits_set".into(),    severity: 10.0, check: |r| r.has_resource_limits },
            ],
        }
    }

    pub fn rule_names(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.name.clone()).collect()
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }

    pub fn audit(&self, resources: &[K8sResource]) -> StaticAuditReport {
        let mut findings = Vec::new();
        let mut total_weight = 0.0_f64;
        let mut failed_weight = 0.0_f64;

        for r in resources {
            for rule in &self.rules {
                let passed = (rule.check)(r);
                total_weight += rule.severity;
                if !passed {
                    failed_weight += rule.severity;
                }
                findings.push(Finding {
                    resource: format!("{}/{}/{}", r.kind, r.namespace, r.name),
                    rule: rule.name.clone(),
                    passed,
                    severity: rule.severity,
                });
            }
        }

        let static_risk = if total_weight > 0.0 {
            (failed_weight / total_weight * 100.0).min(100.0)
        } else {
            0.0
        };

        StaticAuditReport {
            findings,
            static_risk,
            rules_evaluated: self.rules.len(),
        }
    }
}

impl Default for Auditor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bad() -> K8sResource {
        K8sResource {
            kind: "Deployment".into(),
            name: "emr".into(),
            namespace: "healthcare".into(),
            privileged: true,
            run_as_root: true,
            read_only_root_fs: false,
            has_network_policy: false,
            rbac_least_privilege: false,
            image_tag_is_latest: true,
            has_resource_limits: false,
        }
    }

    fn good() -> K8sResource {
        K8sResource {
            kind: "Deployment".into(),
            name: "emr".into(),
            namespace: "healthcare".into(),
            privileged: false,
            run_as_root: false,
            read_only_root_fs: true,
            has_network_policy: true,
            rbac_least_privilege: true,
            image_tag_is_latest: false,
            has_resource_limits: true,
        }
    }

    #[test]
    fn bad_resource_scores_high() {
        let a = Auditor::new();
        let report = a.audit(&[bad()]);
        assert!(report.static_risk > 80.0);
    }

    #[test]
    fn good_resource_scores_zero() {
        let a = Auditor::new();
        let report = a.audit(&[good()]);
        assert!(report.static_risk < 1.0);
    }
}
