use albert_ai_usage_core::format_result;
use albert_ai_usage_core::providers::claude_mock::ClaudeMockProvider;
use albert_ai_usage_core::providers::codex::CodexProvider;
use albert_ai_usage_core::providers::copilot::{self, CopilotProvider};
use albert_ai_usage_core::registry::ProviderRegistry;

fn main() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(ClaudeMockProvider));
    registry.register(Box::new(CodexProvider));

    if std::env::var("ALBERT_LIVE_COPILOT").as_deref() == Ok("1") {
        // One provider per GitHub account gh has stored -- a person
        // can have more than one (e.g. personal + a business seat).
        for provider in copilot::discover_live_providers() {
            registry.register(provider);
        }
    } else {
        registry.register(Box::new(CopilotProvider::fixture()));
    }

    for (id, result) in registry.refresh_all() {
        println!("{}", format_result(&id, &result));
    }
}
