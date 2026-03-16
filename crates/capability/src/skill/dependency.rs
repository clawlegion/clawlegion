//! Skill dependency management - cycle detection and topological sorting

use clawlegion_core::Result;
use std::collections::{HashMap, HashSet, VecDeque};

/// Skill dependency graph
pub struct SkillDependencyGraph {
    /// Adjacency list: skill -> skills that depend on it
    graph: HashMap<String, Vec<String>>,
    /// Reverse mapping: skill -> skills it depends on
    dependencies: HashMap<String, Vec<String>>,
}

impl SkillDependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Add a dependency edge (from depends on to)
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.graph
            .entry(to.to_string())
            .or_default()
            .push(from.to_string());
        self.dependencies
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    /// Add multiple dependencies for a skill
    pub fn add_dependencies(&mut self, skill: &str, deps: Vec<String>) {
        for dep in deps {
            self.add_dependency(skill, &dep);
        }
    }

    /// Check if there's a cycle in the dependency graph
    /// Returns the cycle path if found
    pub fn has_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.graph.keys() {
            if !visited.contains(node) {
                if let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.dfs_cycle(neighbor, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|x| x == neighbor).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }

    /// Get topological sort (execution order)
    /// Returns skills in order such that dependencies come before dependents
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        if let Some(cycle) = self.has_cycle() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Circular dependency detected: {}",
                    cycle.join(" -> ")
                )),
            ));
        }

        // Calculate in-degrees
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for node in self.graph.keys() {
            in_degree.entry(node.clone()).or_insert(0);
        }
        for node in self.dependencies.keys() {
            *in_degree.entry(node.clone()).or_insert(0) +=
                self.dependencies.get(node).map(|v| v.len()).unwrap_or(0);
        }

        // Use Kahn's algorithm
        let mut queue = VecDeque::new();
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(neighbors) = self.graph.get(&node) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get all dependencies of a skill (transitive)
    pub fn get_all_dependencies(&self, skill: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(deps) = self.dependencies.get(skill) {
            for dep in deps {
                queue.push_back(dep.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            if visited.insert(node.clone()) {
                result.push(node.clone());
                if let Some(deps) = self.dependencies.get(&node) {
                    for dep in deps {
                        if !visited.contains(dep) {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        result
    }

    /// Get all skills that depend on a given skill
    pub fn get_dependents(&self, skill: &str) -> Vec<String> {
        self.graph.get(skill).cloned().unwrap_or_default()
    }

    /// Check if a skill can be safely loaded (all dependencies satisfied)
    pub fn can_load(&self, skill: &str, available: &HashSet<String>) -> bool {
        if let Some(deps) = self.dependencies.get(skill) {
            deps.iter().all(|dep| available.contains(dep))
        } else {
            true
        }
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.graph.clear();
        self.dependencies.clear();
    }

    /// Get the number of nodes in the graph
    pub fn len(&self) -> usize {
        self.graph.len()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.graph.is_empty()
    }
}

impl Default for SkillDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_cycle() {
        let mut graph = SkillDependencyGraph::new();
        graph.add_dependency("B", "A"); // B depends on A
        graph.add_dependency("C", "B"); // C depends on B

        assert!(graph.has_cycle().is_none());
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = SkillDependencyGraph::new();
        graph.add_dependency("B", "A"); // B depends on A
        graph.add_dependency("C", "B"); // C depends on B
        graph.add_dependency("A", "C"); // A depends on C (creates cycle)

        let cycle = graph.has_cycle();
        assert!(cycle.is_some());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = SkillDependencyGraph::new();
        graph.add_dependency("B", "A"); // B depends on A
        graph.add_dependency("C", "B"); // C depends on B
        graph.add_dependency("D", "A"); // D depends on A

        let sorted = graph.topological_sort().unwrap();

        // A must come before B and D
        // B must come before C
        let a_pos = sorted.iter().position(|x| x == "A").unwrap();
        let b_pos = sorted.iter().position(|x| x == "B").unwrap();
        let c_pos = sorted.iter().position(|x| x == "C").unwrap();
        let d_pos = sorted.iter().position(|x| x == "D").unwrap();

        assert!(a_pos < b_pos);
        assert!(a_pos < d_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = SkillDependencyGraph::new();
        graph.add_dependency("B", "A");
        graph.add_dependency("C", "B");
        graph.add_dependency("D", "C");

        let deps = graph.get_all_dependencies("D");
        assert!(deps.contains(&"C".to_string()));
        assert!(deps.contains(&"B".to_string()));
        assert!(deps.contains(&"A".to_string()));
        assert_eq!(deps.len(), 3);
    }
}
