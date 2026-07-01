"""
A2X Python Client SDK

A lightweight Python client for the A2X gateway REST API.
Supports program execution, entity listing, agent probing,
and webhook registration.

API Reference:
    POST /a2x/execute        — Execute a Σ∞ program
    GET  /a2x/entities       — List connected entities/agents
    GET  /a2x/entities/:id   — Get entity/agent details
    GET  /a2x/probe/:agent   — Probe agent state
    POST /a2x/webhook        — Register a webhook callback

Usage:
    from a2x_client import A2xClient

    client = A2xClient("http://localhost:8778", api_key="my-key")
    result = client.execute("⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ D:⟘⟭")
    print(result.result)
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Any, Optional

import requests


# ── Data types ──────────────────────────────────────────────────────────────

@dataclass
class ExecuteResponse:
    """Result of executing a Σ∞ program via the gateway."""
    result: str
    execution_time_ms: int
    status: str  # "completed", "error", "timeout"


@dataclass
class EntityInfo:
    """Information about a connected entity or agent."""
    id: str
    entity_type: str
    display_name: str
    capabilities: list[str]


@dataclass
class ProbeResponse:
    """Snapshot of an agent's internal state."""
    agent_id: str
    state: str
    ip: Optional[int]
    world_graph_size: int
    memory_trace_length: int


@dataclass
class WebhookResponse:
    """Result of registering a webhook callback."""
    webhook_id: str


# ── Client ─────────────────────────────────────────────────────────────────

class A2xClient:
    """Client for the A2X gateway REST API.

    Args:
        base_url: Gateway base URL (e.g., "http://localhost:8778").
        api_key: Optional API key for authentication.
        timeout: Request timeout in seconds (default 30).
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8778",
        api_key: Optional[str] = None,
        timeout: float = 30.0,
    ):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout
        self._session = requests.Session()
        if api_key:
            self._session.headers["X-A2X-Key"] = api_key

    # ── Program Execution ──────────────────────────────────────────────

    def execute(
        self,
        program: str,
        format: str = "sigma",
        timeout_ms: int = 5000,
    ) -> ExecuteResponse:
        """Execute a Σ∞ program via the gateway.

        Args:
            program: Σ∞ program source text.
            format: Program format ("sigma" or "omega").
            timeout_ms: Execution timeout in milliseconds.

        Returns:
            ExecuteResponse with result, timing, and status.

        Raises:
            requests.HTTPError: On HTTP errors (4xx, 5xx).
            requests.ConnectionError: If the gateway is unreachable.
        """
        url = f"{self.base_url}/a2x/execute"
        params = {}
        if self.api_key:
            params["api_key"] = self.api_key

        payload = {
            "program": program,
            "format": format,
            "timeout_ms": timeout_ms,
        }

        resp = self._session.post(
            url,
            json=payload,
            params=params,
            timeout=self.timeout,
        )
        resp.raise_for_status()
        data = resp.json()
        return ExecuteResponse(
            result=data["result"],
            execution_time_ms=data["execution_time_ms"],
            status=data["status"],
        )

    # ── Entity Discovery ───────────────────────────────────────────────

    def list_entities(self) -> list[EntityInfo]:
        """List all connected entities and agents.

        Returns:
            List of EntityInfo objects.

        Raises:
            requests.HTTPError: On HTTP errors.
        """
        url = f"{self.base_url}/a2x/entities"
        resp = self._session.get(url, timeout=self.timeout)
        resp.raise_for_status()
        data = resp.json()
        return [
            EntityInfo(
                id=e["id"],
                entity_type=e["entity_type"],
                display_name=e["display_name"],
                capabilities=e["capabilities"],
            )
            for e in data
        ]

    def get_entity(self, entity_id: str) -> EntityInfo:
        """Get details about a specific entity or agent.

        Args:
            entity_id: The entity ID to look up.

        Returns:
            EntityInfo for the requested entity.

        Raises:
            requests.HTTPError: On HTTP errors (404 if not found).
        """
        url = f"{self.base_url}/a2x/entities/{entity_id}"
        resp = self._session.get(url, timeout=self.timeout)
        resp.raise_for_status()
        data = resp.json()
        return EntityInfo(
            id=data["id"],
            entity_type=data["entity_type"],
            display_name=data["display_name"],
            capabilities=data["capabilities"],
        )

    # ── Agent Probing ──────────────────────────────────────────────────

    def probe_agent(self, agent_id: str) -> ProbeResponse:
        """Probe an agent's internal state.

        Args:
            agent_id: The agent ID to probe.

        Returns:
            ProbeResponse with state snapshot.

        Raises:
            requests.HTTPError: On HTTP errors (404 if agent not found, 403 if probing is denied).
        """
        url = f"{self.base_url}/a2x/probe/{agent_id}"
        resp = self._session.get(url, timeout=self.timeout)
        resp.raise_for_status()
        data = resp.json()
        return ProbeResponse(
            agent_id=data["agent_id"],
            state=data["state"],
            ip=data.get("ip"),
            world_graph_size=data["world_graph_size"],
            memory_trace_length=data["memory_trace_length"],
        )

    # ── Webhooks ───────────────────────────────────────────────────────

    def register_webhook(
        self,
        callback_url: str,
        filter_correlation_ids: Optional[list[int]] = None,
    ) -> WebhookResponse:
        """Register a webhook callback for program completion events.

        Args:
            callback_url: URL to POST results to.
            filter_correlation_ids: Optional list of correlation IDs to filter.

        Returns:
            WebhookResponse with the registered webhook ID.
        """
        url = f"{self.base_url}/a2x/webhook"
        payload: dict[str, Any] = {"url": callback_url}
        if filter_correlation_ids:
            payload["filter_correlation_ids"] = filter_correlation_ids

        resp = self._session.post(url, json=payload, timeout=self.timeout)
        resp.raise_for_status()
        data = resp.json()
        return WebhookResponse(webhook_id=data["webhook_id"])

    # ── Health Check ──────────────────────────────────────────────────

    def health(self) -> bool:
        """Check if the gateway is reachable.

        Returns:
            True if the gateway responds, False otherwise.
        """
        try:
            resp = self._session.get(
                f"{self.base_url}/a2x/entities",
                timeout=5.0,
            )
            return resp.ok
        except requests.RequestException:
            return False

    # ── Context Manager ───────────────────────────────────────────────

    def close(self):
        """Close the underlying HTTP session."""
        self._session.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


# ── Convenience Functions ──────────────────────────────────────────────────

def execute(
    program: str,
    base_url: str = "http://localhost:8778",
    api_key: Optional[str] = None,
) -> ExecuteResponse:
    """Quick one-shot program execution.

    Args:
        program: Σ∞ program source text.
        base_url: Gateway base URL.
        api_key: Optional API key.

    Returns:
        ExecuteResponse.
    """
    with A2xClient(base_url, api_key) as client:
        return client.execute(program)
