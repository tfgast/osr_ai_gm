#!/usr/bin/env python3
"""Shared LLM API caller supporting gemini, claude, and codex CLIs as backends.

Each CLI handles authentication and endpoint routing. We invoke them
in headless/print mode and capture the output.

For vision tasks, codex uses native -i flag; gemini and claude get a
prompt prefix instructing the agent to read the image file.

Model routing:
  - Models starting with "claude-" or aliases "opus", "sonnet", "haiku"
    use the claude CLI.
  - Models starting with "gpt-", "o1", "o3", "o4", or "chatgpt-"
    use the codex CLI.
  - All other models use the gemini CLI.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path


def extract_json_from_text(text: str) -> dict | None:
    """Extract JSON from text that may have markdown fences or other content."""
    text = text.strip()

    # Try direct parse
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass

    # Try stripping markdown code fences
    m = re.search(r"```(?:json)?\s*\n(.*?)```", text, re.DOTALL)
    if m:
        try:
            return json.loads(m.group(1).strip())
        except json.JSONDecodeError:
            pass

    # Try finding outermost braces
    first = text.find("{")
    last = text.rfind("}")
    if first != -1 and last > first:
        try:
            return json.loads(text[first : last + 1])
        except json.JSONDecodeError:
            pass

    return None


def _is_claude_model(model: str) -> bool:
    """Check if the model should use the claude CLI."""
    claude_prefixes = ("claude-", "opus", "sonnet", "haiku")
    return model.lower().startswith(claude_prefixes)


def _is_openai_model(model: str) -> bool:
    """Check if the model should use the codex CLI."""
    openai_prefixes = ("gpt-", "o1", "o3", "o4", "chatgpt-")
    return model.lower().startswith(openai_prefixes)


def _call_gemini_cli(full_prompt: str, model: str) -> str:
    """Call the gemini CLI in headless mode."""
    cmd = [
        "gemini",
        "-m", model,
        "-o", "text",
        "-y",               # auto-approve file reads
        "-p", "",           # headless mode, prompt from stdin
    ]

    print(f"  Calling gemini CLI (model={model})...", file=sys.stderr)

    result = subprocess.run(
        cmd,
        input=full_prompt,
        capture_output=True,
        text=True,
        timeout=600,
    )

    if result.returncode != 0:
        print(f"gemini CLI error (exit {result.returncode}):", file=sys.stderr)
        print(result.stderr[:1000], file=sys.stderr)
        raise RuntimeError(f"gemini CLI failed with exit code {result.returncode}")

    output = result.stdout.strip()
    if not output:
        raise ValueError("gemini CLI returned empty output")

    return output


def _call_claude_cli(full_prompt: str, model: str) -> str:
    """Call the claude CLI in print mode."""
    # Strip CLAUDECODE env var to avoid nesting check
    env = {k: v for k, v in os.environ.items() if k != "CLAUDECODE"}

    cmd = [
        "claude",
        "-p",                       # print mode
        "--model", model,
        "--output-format", "text",
        "--allowedTools", "Read",   # allow reading image files
        "--dangerously-skip-permissions",
    ]

    print(f"  Calling claude CLI (model={model})...", file=sys.stderr)

    result = subprocess.run(
        cmd,
        input=full_prompt,
        capture_output=True,
        text=True,
        timeout=600,
        env=env,
    )

    if result.returncode != 0:
        print(f"claude CLI error (exit {result.returncode}):", file=sys.stderr)
        print(result.stderr[:1000], file=sys.stderr)
        raise RuntimeError(f"claude CLI failed with exit code {result.returncode}")

    output = result.stdout.strip()
    if not output:
        raise ValueError("claude CLI returned empty output")

    return output


def _call_codex_cli(full_prompt: str, model: str, image_path: str | None = None) -> str:
    """Call the codex CLI in non-interactive exec mode."""
    cmd = [
        "codex", "exec",
        "-m", model,
        "--ephemeral",
        "--dangerously-bypass-approvals-and-sandbox",
        "-",                    # read prompt from stdin
    ]
    if image_path:
        cmd[2:2] = ["-i", str(Path(image_path).resolve())]

    print(f"  Calling codex CLI (model={model})...", file=sys.stderr)

    result = subprocess.run(
        cmd,
        input=full_prompt,
        capture_output=True,
        text=True,
        timeout=600,
    )

    if result.returncode != 0:
        print(f"codex CLI error (exit {result.returncode}):", file=sys.stderr)
        print(result.stderr[:1000], file=sys.stderr)
        raise RuntimeError(f"codex CLI failed with exit code {result.returncode}")

    output = result.stdout.strip()
    if not output:
        raise ValueError("codex CLI returned empty output")

    return output


def call_llm(
    prompt: str,
    image_path: str | None = None,
    model: str = "gemini-2.5-pro",
) -> str:
    """Call an LLM via its CLI in headless mode.

    Args:
        prompt: The text prompt to send.
        image_path: Optional path to an image file. Codex handles this
            natively via -i; gemini/claude get a prompt prefix to read it.
        model: Model name. Routes to claude, codex, or gemini CLI.

    Returns:
        Raw response text from the model.
    """
    # Codex handles images natively via -i flag
    if _is_openai_model(model):
        full_prompt = (
            f"{prompt}\n\n"
            f"Output ONLY the JSON object, no markdown fences, no commentary."
        )
        return _call_codex_cli(full_prompt, model, image_path=image_path)

    # Gemini and Claude: embed image path in prompt
    if image_path:
        abs_path = str(Path(image_path).resolve())
        full_prompt = (
            f"First, read the image file at: {abs_path}\n"
            f"Then follow these instructions:\n\n{prompt}\n\n"
            f"Output ONLY the JSON object, no markdown fences, no commentary."
        )
    else:
        full_prompt = (
            f"{prompt}\n\n"
            f"Output ONLY the JSON object, no markdown fences, no commentary."
        )

    if _is_claude_model(model):
        return _call_claude_cli(full_prompt, model)
    else:
        return _call_gemini_cli(full_prompt, model)


def call_llm_json(
    prompt: str,
    image_path: str | None = None,
    model: str = "gemini-2.5-pro",
) -> dict:
    """Call an LLM and parse the response as JSON.

    Returns parsed dict, or raises ValueError if JSON extraction fails.
    """
    raw = call_llm(
        prompt=prompt,
        image_path=image_path,
        model=model,
    )

    result = extract_json_from_text(raw)
    if result is None:
        # Save raw output for debugging
        raw_path = Path("llm_raw_output.txt")
        raw_path.write_text(raw)
        raise ValueError(
            f"Could not parse JSON from response (saved to {raw_path}):\n{raw[:2000]}"
        )

    return result
