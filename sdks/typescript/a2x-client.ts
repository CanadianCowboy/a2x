// A2X TypeScript/JavaScript Client SDK
//
// A lightweight client for the A2X gateway REST API.
// Supports program execution, entity listing, agent probing,
// and webhook registration.
//
// Usage:
//   import { A2xClient } from "./a2x-client";
//
//   const client = new A2xClient("http://localhost:8778", "my-key");
//   const result = await client.execute("⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ D:⟘⟭");
//   console.log(result.result);

// ── Data types ──────────────────────────────────────────────────────────────

/** Result of executing a Σ∞ program via the gateway. */
export interface ExecuteResponse {
  /** Result Σ∞ program text. */
  result: string;
  /** Execution time in milliseconds. */
  execution_time_ms: number;
  /** Status: "completed", "error", or "timeout". */
  status: string;
}

/** Information about a connected entity or agent. */
export interface EntityInfo {
  /** Unique entity identifier. */
  id: string;
  /** Entity type classification. */
  entity_type: string;
  /** Human-readable display name. */
  display_name: string;
  /** Capabilities this entity provides. */
  capabilities: string[];
}

/** Snapshot of an agent's internal state. */
export interface ProbeResponse {
  /** Agent identifier. */
  agent_id: string;
  /** Current agent state (e.g., "idle", "running"). */
  state: string;
  /** Instruction pointer (null if VM has no program loaded). */
  ip: number | null;
  /** Number of nodes in the WorldGraph. */
  world_graph_size: number;
  /** Number of entries in the MemoryTrace. */
  memory_trace_length: number;
}

/** Result of registering a webhook callback. */
export interface WebhookResponse {
  /** Registered webhook identifier. */
  webhook_id: string;
}

// ── Client ─────────────────────────────────────────────────────────────────

export class A2xClient {
  private baseUrl: string;
  private apiKey?: string;
  private timeout: number;

  /**
   * Create a new A2X gateway client.
   *
   * @param baseUrl - Gateway base URL (default: "http://localhost:8778")
   * @param apiKey - Optional API key for authentication
   * @param timeout - Request timeout in milliseconds (default: 30000)
   */
  constructor(
    baseUrl: string = "http://localhost:8778",
    apiKey?: string,
    timeout: number = 30000,
  ) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.apiKey = apiKey;
    this.timeout = timeout;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    params?: Record<string, string>,
  ): Promise<T> {
    const url = new URL(path, this.baseUrl);
    if (params) {
      for (const [key, value] of Object.entries(params)) {
        url.searchParams.set(key, value);
      }
    }

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.apiKey) {
      headers["X-A2X-Key"] = this.apiKey;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(url.toString(), {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(
          `A2X API error ${response.status}: ${errorBody}`,
        );
      }

      return (await response.json()) as T;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  // ── Program Execution ──────────────────────────────────────────────

  /**
   * Execute a Σ∞ program via the gateway.
   *
   * @param program - Σ∞ program source text
   * @param format - Program format ("sigma" or "omega", default "sigma")
   * @param timeoutMs - Execution timeout in milliseconds (default 5000)
   * @returns ExecuteResponse with result, timing, and status
   */
  async execute(
    program: string,
    format: string = "sigma",
    timeoutMs: number = 5000,
  ): Promise<ExecuteResponse> {
    const params: Record<string, string> = {};
    if (this.apiKey) {
      params["api_key"] = this.apiKey;
    }

    return this.request<ExecuteResponse>("POST", "/a2x/execute", {
      program,
      format,
      timeout_ms: timeoutMs,
    }, params);
  }

  // ── Entity Discovery ───────────────────────────────────────────────

  /**
   * List all connected entities and agents.
   *
   * @returns Array of EntityInfo objects
   */
  async listEntities(): Promise<EntityInfo[]> {
    return this.request<EntityInfo[]>("GET", "/a2x/entities");
  }

  /**
   * Get details about a specific entity or agent.
   *
   * @param entityId - The entity ID to look up
   * @returns EntityInfo for the requested entity
   */
  async getEntity(entityId: string): Promise<EntityInfo> {
    return this.request<EntityInfo>(
      "GET",
      `/a2x/entities/${encodeURIComponent(entityId)}`,
    );
  }

  // ── Agent Probing ──────────────────────────────────────────────────

  /**
   * Probe an agent's internal state.
   *
   * @param agentId - The agent ID to probe
   * @returns ProbeResponse with state snapshot
   */
  async probeAgent(agentId: string): Promise<ProbeResponse> {
    return this.request<ProbeResponse>(
      "GET",
      `/a2x/probe/${encodeURIComponent(agentId)}`,
    );
  }

  // ── Webhooks ───────────────────────────────────────────────────────

  /**
   * Register a webhook callback for program completion events.
   *
   * @param callbackUrl - URL to POST results to
   * @param correlationIds - Optional list of correlation IDs to filter
   * @returns WebhookResponse with the registered webhook ID
   */
  async registerWebhook(
    callbackUrl: string,
    correlationIds?: number[],
  ): Promise<WebhookResponse> {
    const body: Record<string, unknown> = { url: callbackUrl };
    if (correlationIds) {
      body["filter_correlation_ids"] = correlationIds;
    }

    return this.request<WebhookResponse>("POST", "/a2x/webhook", body);
  }

  // ── Health Check ──────────────────────────────────────────────────

  /**
   * Check if the gateway is reachable.
   *
   * @returns True if the gateway responds, false otherwise
   */
  async health(): Promise<boolean> {
    try {
      const resp = await fetch(`${this.baseUrl}/a2x/entities`, {
        method: "GET",
        signal: AbortSignal.timeout(5000),
      });
      return resp.ok;
    } catch {
      return false;
    }
  }
}

// ── Convenience Function ──────────────────────────────────────────────────

/**
 * Quick one-shot program execution.
 *
 * @param program - Σ∞ program source text
 * @param baseUrl - Gateway base URL (default: "http://localhost:8778")
 * @param apiKey - Optional API key
 * @returns ExecuteResponse
 */
export async function execute(
  program: string,
  baseUrl: string = "http://localhost:8778",
  apiKey?: string,
): Promise<ExecuteResponse> {
  const client = new A2xClient(baseUrl, apiKey);
  return client.execute(program);
}

export default A2xClient;
