import Foundation

/// Minimal stdio JSON-RPC 2.0 handler for the single MCP `usage` tool.
public struct McpServer {
    private let load: () throws -> Data
    private let now: () -> Date

    public init(load: @escaping () throws -> Data, now: @escaping () -> Date = Date.init) {
        self.load = load
        self.now = now
    }

    /// Processes one newline-delimited JSON-RPC message; returns `nil` for blank lines and notifications.
    public func reply(to line: String) -> String? {
        let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        let parsed: Any
        do {
            parsed = try JSONSerialization.jsonObject(with: Data(trimmed.utf8), options: [.fragmentsAllowed])
        } catch {
            return encodeError(id: NSNull(), code: -32700, message: "Parse error")
        }
        guard let object = parsed as? [String: Any] else {
            return encodeError(id: NSNull(), code: -32600, message: "Invalid Request")
        }
        guard let id = object["id"] else { return nil }
        guard let method = object["method"] as? String else {
            return encodeError(id: id, code: -32600, message: "Invalid Request")
        }

        switch method {
        case "initialize":
            return encodeResult(id: id, result: [
                "protocolVersion": "2024-11-05",
                "capabilities": ["tools": [:] as [String: Any]],
                "serverInfo": ["name": "aiusage", "version": AppVersion.current],
            ])
        case "tools/list":
            return encodeResult(id: id, result: [
                "tools": [[
                    "name": "usage",
                    "description": "Remaining quota per AI provider, most headroom first. Use it to choose where to route work.",
                    "inputSchema": ["type": "object", "properties": [:] as [String: Any]] as [String: Any],
                ]],
            ])
        case "tools/call":
            let params = object["params"] as? [String: Any]
            guard (params?["name"] as? String) == "usage" else {
                return encodeError(id: id, code: -32602, message: "Unknown tool")
            }
            return callUsage(id: id)
        default:
            return encodeError(id: id, code: -32601, message: "Method not found")
        }
    }

    private func callUsage(id: Any) -> String {
        let data: Data
        do {
            data = try load()
        } catch {
            let text = Self.isNotFound(error)
                ? "AIUsage app is not running or has no snapshot yet"
                : error.localizedDescription
            return encodeResult(id: id, result: [
                "content": [["type": "text", "text": text]],
                "isError": true,
            ])
        }
        let snapshot: UsageSnapshot
        do {
            snapshot = try UsageSnapshot.decode(data)
        } catch {
            return encodeResult(id: id, result: [
                "content": [["type": "text", "text": error.localizedDescription]],
                "isError": true,
            ])
        }
        let payload = snapshot.view(now: now())
        guard let text = encodeJSON(payload) else {
            return encodeResult(id: id, result: [
                "content": [["type": "text", "text": "Failed to encode usage snapshot"]],
                "isError": true,
            ])
        }
        return encodeResult(id: id, result: [
            "content": [["type": "text", "text": text]],
        ])
    }

    private static func isNotFound(_ error: Error) -> Bool {
        let ns = error as NSError
        return (ns.domain == NSCocoaErrorDomain && ns.code == NSFileReadNoSuchFileError)
            || (ns.domain == NSPOSIXErrorDomain && ns.code == Int(ENOENT))
    }

    private func encodeResult(id: Any, result: [String: Any]) -> String {
        encodeJSON(["jsonrpc": "2.0", "id": id, "result": result]) ?? "{}"
    }

    private func encodeError(id: Any, code: Int, message: String) -> String {
        encodeJSON(["jsonrpc": "2.0", "id": id, "error": ["code": code, "message": message]]) ?? "{}"
    }

    private func encodeJSON(_ value: Any) -> String? {
        guard JSONSerialization.isValidJSONObject(value),
              let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys, .withoutEscapingSlashes])
        else { return nil }
        return String(data: data, encoding: .utf8)
    }
}
