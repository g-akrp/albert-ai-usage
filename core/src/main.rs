use albert_ai_usage_core::format_result;
use albert_ai_usage_core::providers::claude_mock::ClaudeMockProvider;
use albert_ai_usage_core::registry::ProviderRegistry;

fn main() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(ClaudeMockProvider));

    for (id, result) in registry.refresh_all() {
        println!("{}", format_result(&id, &result));
    }
}
