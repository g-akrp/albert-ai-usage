pub fn default_order(ids: &[String]) -> Vec<String> {
    let mut result = ids.to_vec();
    let preferred = ["claude", "codex", "antigravity", "copilot"];
    result.sort_by_key(|s| {
        preferred
            .iter()
            .position(|p| *p == s.split(':').next().unwrap_or(s))
            .unwrap_or(4)
    });
    result
}
pub fn reconcile(saved: &[String], ids: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for id in saved.iter().chain(default_order(ids).iter()) {
        if ids.contains(id) && !result.contains(id) {
            result.push(id.clone());
        }
    }
    result
}
pub fn move_visible(order: &mut [String], id: &str, step: isize, hidden: &[String]) {
    let visible: Vec<usize> = order
        .iter()
        .enumerate()
        .filter(|(_, s)| !hidden.contains(s))
        .map(|(i, _)| i)
        .collect();
    let Some(index) = visible.iter().position(|i| order[*i] == id) else {
        return;
    };
    let next = index as isize + step;
    if next >= 0 && (next as usize) < visible.len() {
        order.swap(visible[index], visible[next as usize]);
    }
}
