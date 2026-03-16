//! Organization Tree - hierarchical company structure

use crate::OrgAgent;
use clawlegion_core::{AgentId, CompanyId, Error, OrgError, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Organization Tree
///
/// Represents the hierarchical structure of a company.
/// Each company has exactly one org tree with a CEO at the root.
pub struct OrgTree {
    /// Company ID this tree belongs to
    company_id: CompanyId,

    /// All agents in the org (indexed by ID)
    agents: DashMap<AgentId, Arc<RwLock<OrgAgent>>>,

    /// Agent ID of the CEO (root of the tree)
    ceo_id: RwLock<Option<AgentId>>,

    /// Direct reports mapping (manager_id -> list of direct reports)
    direct_reports: DashMap<AgentId, Vec<AgentId>>,
}

impl OrgTree {
    /// Create a new org tree for a company
    pub fn new(company_id: CompanyId) -> Self {
        Self {
            company_id,
            agents: DashMap::new(),
            ceo_id: RwLock::new(None),
            direct_reports: DashMap::new(),
        }
    }

    /// Add an agent to the org tree
    pub fn add_agent(&self, agent: OrgAgent) -> Result<()> {
        if agent.company_id != self.company_id {
            return Err(Error::Org(OrgError::InvalidStructure(
                "Agent company ID does not match org tree company ID".to_string(),
            )));
        }

        let agent_id = agent.id;
        let reports_to = agent.reports_to;

        // Check for cycles
        if let Some(manager_id) = reports_to {
            if self.would_create_cycle(manager_id, agent_id) {
                return Err(Error::Org(OrgError::CycleDetected));
            }

            // Add to manager's direct reports
            self.direct_reports
                .entry(manager_id)
                .or_default()
                .push(agent_id);
        } else {
            // This is the CEO
            if self.ceo_id.read().is_some() {
                return Err(Error::Org(OrgError::InvalidStructure(
                    "Org tree already has a CEO".to_string(),
                )));
            }
            *self.ceo_id.write() = Some(agent_id);
        }

        let arc_agent = Arc::new(RwLock::new(agent));
        self.agents.insert(agent_id, arc_agent);

        Ok(())
    }

    /// Remove an agent from the org tree
    pub fn remove_agent(&self, agent_id: AgentId) -> Result<()> {
        // Check if this agent has direct reports
        if let Some(reports) = self.direct_reports.get(&agent_id) {
            if !reports.is_empty() {
                return Err(Error::Org(OrgError::InvalidStructure(
                    "Cannot remove agent with direct reports. Reassign them first.".to_string(),
                )));
            }
        }

        // Remove from direct_reports
        self.direct_reports.remove(&agent_id);

        // Remove from any manager's direct reports list
        for mut entry in self.direct_reports.iter_mut() {
            entry.value_mut().retain(|&id| id != agent_id);
        }

        // Remove from agents
        self.agents
            .remove(&agent_id)
            .ok_or_else(|| Error::Org(OrgError::AgentNotFound(agent_id.to_string())))?;

        // Update CEO if necessary
        if *self.ceo_id.read() == Some(agent_id) {
            *self.ceo_id.write() = None;
        }

        Ok(())
    }

    /// Get an agent by ID
    pub fn get_agent(&self, agent_id: AgentId) -> Option<Arc<RwLock<OrgAgent>>> {
        self.agents.get(&agent_id).map(|entry| entry.clone())
    }

    /// Get the CEO
    pub fn get_ceo(&self) -> Option<Arc<RwLock<OrgAgent>>> {
        self.ceo_id.read().and_then(|id| self.get_agent(id))
    }

    /// Get direct reports for an agent
    pub fn get_direct_reports(&self, manager_id: AgentId) -> Vec<Arc<RwLock<OrgAgent>>> {
        self.direct_reports
            .get(&manager_id)
            .map(|reports| {
                reports
                    .iter()
                    .filter_map(|&id| self.get_agent(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all agents under a manager (recursive)
    pub fn get_all_reports(&self, manager_id: AgentId) -> Vec<Arc<RwLock<OrgAgent>>> {
        let mut result = vec![];
        let mut queue: VecDeque<AgentId> = VecDeque::new();

        // Start with direct reports
        if let Some(reports) = self.direct_reports.get(&manager_id) {
            for &report_id in reports.iter() {
                queue.push_back(report_id);
            }
        }

        // BFS traversal
        while let Some(current_id) = queue.pop_front() {
            if let Some(agent) = self.get_agent(current_id) {
                result.push(agent.clone());

                // Add this agent's direct reports to queue
                if let Some(reports) = self.direct_reports.get(&current_id) {
                    for &report_id in reports.iter() {
                        queue.push_back(report_id);
                    }
                }
            }
        }

        result
    }

    /// Get the manager of an agent
    pub fn get_manager(&self, agent_id: AgentId) -> Option<Arc<RwLock<OrgAgent>>> {
        self.agents
            .get(&agent_id)
            .and_then(|entry| entry.read().reports_to)
            .and_then(|manager_id| self.get_agent(manager_id))
    }

    /// Get the chain of command (from agent up to CEO)
    pub fn get_chain_of_command(&self, agent_id: AgentId) -> Vec<Arc<RwLock<OrgAgent>>> {
        let mut chain = vec![];
        let mut current_id = Some(agent_id);

        while let Some(id) = current_id {
            if let Some(agent) = self.get_agent(id) {
                chain.push(agent.clone());
                current_id = agent.read().reports_to;
            } else {
                break;
            }
        }

        chain
    }

    /// Check if an agent is in the chain of command of another agent
    pub fn is_in_chain(&self, superior_id: AgentId, subordinate_id: AgentId) -> bool {
        let chain = self.get_chain_of_command(subordinate_id);
        chain.iter().any(|agent| agent.read().id == superior_id)
    }

    /// Get the depth of an agent in the org tree (CEO = 0)
    pub fn get_depth(&self, agent_id: AgentId) -> Option<usize> {
        let chain = self.get_chain_of_command(agent_id);
        if chain.is_empty() {
            None
        } else {
            Some(chain.len() - 1)
        }
    }

    /// Get all agents in the org
    pub fn get_all_agents(&self) -> Vec<Arc<RwLock<OrgAgent>>> {
        self.agents.iter().map(|entry| entry.clone()).collect()
    }

    /// Get the total number of agents
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Check if adding a report would create a cycle
    fn would_create_cycle(&self, manager_id: AgentId, new_report_id: AgentId) -> bool {
        // If the new report is already an ancestor of the manager, it would create a cycle
        let chain = self.get_chain_of_command(manager_id);
        chain.iter().any(|agent| agent.read().id == new_report_id)
    }

    /// Get the org chart as a nested structure
    pub fn get_org_chart(&self) -> Option<OrgNode> {
        self.ceo_id
            .read()
            .and_then(|ceo_id| self.get_agent(ceo_id).map(|ceo| self.build_org_node(ceo)))
    }

    fn build_org_node(&self, agent: Arc<RwLock<OrgAgent>>) -> OrgNode {
        let agent_read = agent.read();

        let reports = self
            .get_direct_reports(agent_read.id)
            .into_iter()
            .map(|report_agent| self.build_org_node(report_agent))
            .collect();

        OrgNode {
            id: agent_read.id,
            name: agent_read.name.clone(),
            role: agent_read.role.clone(),
            title: agent_read.title.clone(),
            status: agent_read.status,
            reports,
        }
    }

    /// Get company ID
    pub fn company_id(&self) -> CompanyId {
        self.company_id
    }
}

/// Organization node for chart representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgNode {
    pub id: AgentId,
    pub name: String,
    pub role: String,
    pub title: String,
    pub status: clawlegion_core::AgentStatus,
    pub reports: Vec<OrgNode>,
}

/// Builder for creating org trees from configuration
pub struct OrgTreeBuilder {
    company_id: CompanyId,
    agents: Vec<OrgAgent>,
}

impl OrgTreeBuilder {
    pub fn new(company_id: CompanyId) -> Self {
        Self {
            company_id,
            agents: vec![],
        }
    }

    pub fn add_agent(mut self, agent: OrgAgent) -> Self {
        self.agents.push(agent);
        self
    }

    pub fn build(self) -> Result<OrgTree> {
        let tree = OrgTree::new(self.company_id);

        // First pass: add all agents
        for agent in self.agents {
            tree.add_agent(agent)?;
        }

        Ok(tree)
    }
}
