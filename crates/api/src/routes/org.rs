//! Organization routes

use axum::{extract::State, Json};

use crate::{
    dto::{
        BudgetResponse, CompanyResponse, ListOrgAgentsResponse, OrgAgentResponse, OrgNodeResponse,
        OrgTreeResponse,
    },
    state::ApiState,
};

/// Get company info
pub async fn get_company(State(state): State<ApiState>) -> Json<CompanyResponse> {
    let company_id = state.org_tree.company_id();
    let agent_count = state.org_tree.agent_count();

    // Placeholder values - would need to query company config
    Json(CompanyResponse {
        company_id: company_id.to_string(),
        company_name: "Unknown Company".to_string(), // TODO: Get from company config
        issue_prefix: "UNKNOWN".to_string(),         // TODO: Get from company config
        budget_monthly_cents: 0,                     // TODO: Get from company config
        budget_spent_cents: 0,                       // TODO: Calculate from agent usage
        agent_count,
        created_at: None,
    })
}

/// Get organization tree
pub async fn get_org_tree(State(state): State<ApiState>) -> Json<OrgTreeResponse> {
    let chart = state.org_tree.get_org_chart();

    let root = chart.map(|node| build_org_node_response(&node, 0));

    Json(OrgTreeResponse { root })
}

/// List all agents in org (flat list)
pub async fn list_org_agents(State(state): State<ApiState>) -> Json<ListOrgAgentsResponse> {
    let agents = state.org_tree.get_all_agents();

    let agent_responses: Vec<OrgAgentResponse> = agents
        .iter()
        .map(|agent_arc| {
            let agent = agent_arc.read();
            let depth = state.org_tree.get_depth(agent.id).unwrap_or(0) as u32;
            let direct_reports_count = state.org_tree.get_direct_reports(agent.id).len();

            OrgAgentResponse {
                id: agent.id.to_string(),
                name: agent.name.clone(),
                role: agent.role.clone(),
                title: agent.title.clone(),
                depth,
                parent_id: agent.reports_to.map(|id| id.to_string()),
                direct_reports_count,
            }
        })
        .collect();

    Json(ListOrgAgentsResponse {
        agents: agent_responses,
    })
}

/// Get budget status
pub async fn get_budget(State(state): State<ApiState>) -> Json<BudgetResponse> {
    let company_id = state.org_tree.company_id();

    // Placeholder values
    let budget_monthly_cents: u64 = 1_000_000;
    let budget_spent_cents: u64 = 350_000;
    let budget_remaining_cents = budget_monthly_cents - budget_spent_cents;
    let usage_percentage = (budget_spent_cents as f64 / budget_monthly_cents as f64) * 100.0;

    Json(BudgetResponse {
        company_id: company_id.to_string(),
        budget_monthly_cents,
        budget_spent_cents,
        budget_remaining_cents,
        usage_percentage,
        projected_overrun: false,
        top_spenders: vec![], // TODO: Calculate from agent usage
    })
}

fn build_org_node_response(node: &clawlegion_org::OrgNode, depth: u32) -> OrgNodeResponse {
    OrgNodeResponse {
        node_id: node.id.to_string(),
        name: node.name.clone(),
        role: node.role.clone(),
        title: node.title.clone(),
        icon: None, // TODO: Get from agent config
        depth,
        children: node
            .reports
            .iter()
            .map(|child| build_org_node_response(child, depth + 1))
            .collect(),
    }
}
