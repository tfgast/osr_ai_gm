#!/usr/bin/env python3
"""
Direct Gemini Vision API call for dungeon map room connection extraction.

Uses the OAuth credentials from the gemini CLI (~/.gemini/oauth_creds.json)
to make authenticated requests to the Gemini API with image input.

Usage:
    python vision_extract.py <map_image> [model] [output_json]

    model defaults to gemini-2.5-pro
"""

import base64
import json
import os
import sys
import time
from pathlib import Path

# Try google-genai SDK first, fall back to raw HTTP
try:
    from google import genai
    from google.genai import types
    HAS_SDK = True
except ImportError:
    HAS_SDK = False

import subprocess


EXTRACTION_PROMPT = """You are analyzing a dungeon map image from a tabletop RPG module called "Morkaal's Tomb." Your task is to identify all numbered rooms/areas and map the physical connections (exits/passages) between them.

IMPORTANT: Look carefully at the actual numbers printed on the map. The rooms are numbered with labels like 1, 2, 3A, 3B, 3C, 4, 4A, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15. Do NOT invent room numbers that don't appear on the map.

Instructions:
1. Identify every numbered label on the map image
2. For each numbered area, trace all visible physical connections (doors, passages, stairs, tunnels) to other numbered areas
3. Classify each connection type

Connection Types:
- "door" — a doorway shown as a gap in wall with door marks or arc
- "open" — an open passage, archway, or tunnel with no door
- "stairs" — stairs (shown as parallel lines)
- "secret" — a secret door (marked with "S" on the map)
- "river_crossing" — must cross water/river to reach

Map Symbols:
- "T" = trap
- "S" = secret door
- "PP" = pit trap
- Numbers identify rooms/areas
- A river flows through parts of the complex

Return ONLY a JSON object with this structure:
{
  "rooms": {
    "<room_number>": {
      "name_guess": "<brief visual description of the room>",
      "exits": [
        {
          "to": "<destination room number as shown on map>",
          "connection_type": "<door|open|stairs|secret|river_crossing>",
          "notes": "<observations>"
        }
      ]
    }
  },
  "observations": "<general notes about the map>"
}

Use room numbers EXACTLY as printed on the map. Do not fabricate rooms."""


def get_access_token() -> str:
    """Get a fresh access token from the gemini CLI's OAuth credentials."""
    creds_path = Path.home() / ".gemini" / "oauth_creds.json"
    if not creds_path.exists():
        raise FileNotFoundError("No gemini CLI OAuth creds found at ~/.gemini/oauth_creds.json")

    with open(creds_path) as f:
        creds = json.load(f)

    # Check if token is still valid (with 60s buffer)
    expiry = creds.get("expiry_date", 0)
    if isinstance(expiry, (int, float)):
        # expiry_date is in milliseconds
        if expiry / 1000 > time.time() + 60:
            return creds["access_token"]

    # Token expired — try to refresh using gcloud or google-auth
    print("Access token expired, attempting refresh...", file=sys.stderr)

    # Try using the google-auth library to refresh
    try:
        from google.auth.transport.requests import Request
        from google.oauth2.credentials import Credentials

        # Client ID/secret from gemini CLI config or environment
        client_id = os.environ.get("GOOGLE_CLIENT_ID", "")
        client_secret = os.environ.get("GOOGLE_CLIENT_SECRET", "")
        if not client_id or not client_secret:
            raise ValueError(
                "Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET env vars "
                "for OAuth token refresh"
            )

        credential = Credentials(
            token=creds.get("access_token"),
            refresh_token=creds.get("refresh_token"),
            token_uri="https://oauth2.googleapis.com/token",
            client_id=client_id,
            client_secret=client_secret,
        )
        credential.refresh(Request())

        # Update the cached creds
        creds["access_token"] = credential.token
        creds["expiry_date"] = int(credential.expiry.timestamp() * 1000) if credential.expiry else 0
        with open(creds_path, "w") as f:
            json.dump(creds, f)

        return credential.token
    except Exception as e:
        print(f"Failed to refresh with google-auth: {e}", file=sys.stderr)

    # Fallback: return existing token and hope it works
    return creds["access_token"]


def call_gemini_rest(image_path: str, model: str = "gemini-2.5-pro") -> dict:
    """Call Gemini API directly via REST with OAuth token."""
    token = get_access_token()

    # Read and base64-encode the image
    with open(image_path, "rb") as f:
        image_data = base64.b64encode(f.read()).decode("utf-8")

    # Determine MIME type
    suffix = Path(image_path).suffix.lower()
    mime = {"png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg"}.get(
        suffix.lstrip("."), "image/png"
    )

    # Build the request
    request_body = {
        "contents": [
            {
                "parts": [
                    {"inline_data": {"mime_type": mime, "data": image_data}},
                    {"text": EXTRACTION_PROMPT},
                ]
            }
        ],
        "generationConfig": {
            "temperature": 0.1,
            "maxOutputTokens": 8192,
            "responseMimeType": "application/json",
        },
    }

    # Use the same internal endpoint as the gemini CLI
    url = "https://cloudcode-pa.googleapis.com/v1internal/models/" + model + ":generateContent"

    import urllib.request
    import urllib.error

    req = urllib.request.Request(
        url,
        data=json.dumps(request_body).encode(),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {token}",
        },
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            result = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        print(f"API error {e.code}: {body}", file=sys.stderr)
        raise

    # Extract the text from the response
    candidates = result.get("candidates", [])
    if not candidates:
        raise ValueError(f"No candidates in response: {json.dumps(result, indent=2)}")

    text = candidates[0].get("content", {}).get("parts", [{}])[0].get("text", "")
    return text


def call_gemini_sdk(image_path: str, model: str = "gemini-2.5-pro") -> str:
    """Call Gemini API using the google-genai SDK with OAuth."""
    token = get_access_token()

    client = genai.Client(
        vertexai=False,
        api_key=None,
        http_options=types.HttpOptions(
            headers={"Authorization": f"Bearer {token}"}
        ),
    )

    # Read image
    with open(image_path, "rb") as f:
        image_bytes = f.read()

    suffix = Path(image_path).suffix.lower()
    mime = {"png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg"}.get(
        suffix.lstrip("."), "image/png"
    )

    response = client.models.generate_content(
        model=model,
        contents=[
            types.Part.from_bytes(data=image_bytes, mime_type=mime),
            EXTRACTION_PROMPT,
        ],
        config=types.GenerateContentConfig(
            temperature=0.1,
            max_output_tokens=8192,
            response_mime_type="application/json",
        ),
    )

    return response.text


def extract_json_from_text(text: str) -> dict | None:
    """Extract JSON from text that may have markdown fences."""
    import re

    text = text.strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass

    m = re.search(r"```(?:json)?\s*\n(.*?)```", text, re.DOTALL)
    if m:
        try:
            return json.loads(m.group(1).strip())
        except json.JSONDecodeError:
            pass

    first = text.find("{")
    last = text.rfind("}")
    if first != -1 and last > first:
        try:
            return json.loads(text[first : last + 1])
        except json.JSONDecodeError:
            pass

    return None


def main():
    if len(sys.argv) < 2:
        print("Usage: vision_extract.py <map_image> [model] [output_json]", file=sys.stderr)
        sys.exit(1)

    image_path = sys.argv[1]
    model = sys.argv[2] if len(sys.argv) > 2 else "gemini-2.5-pro"
    output_path = sys.argv[3] if len(sys.argv) > 3 else None

    if not Path(image_path).exists():
        print(f"Error: image not found: {image_path}", file=sys.stderr)
        sys.exit(1)

    print(f"Model: {model}")
    print(f"Image: {image_path}")
    print(f"Using {'SDK' if HAS_SDK else 'REST'} API...")

    try:
        # Always use REST — SDK requires API key, we have OAuth
        raw_text = call_gemini_rest(image_path, model)
    except Exception as e:
        print(f"API call failed: {e}", file=sys.stderr)
        sys.exit(1)

    print(f"Response length: {len(raw_text)} chars")

    result = extract_json_from_text(raw_text)
    if result is None:
        print("WARNING: Could not parse JSON from response", file=sys.stderr)
        print("Raw response:", file=sys.stderr)
        print(raw_text[:2000], file=sys.stderr)
        if output_path:
            with open(output_path + ".raw", "w") as f:
                f.write(raw_text)
        sys.exit(1)

    rooms = result.get("rooms", {})
    print(f"Rooms found: {len(rooms)}")
    print(f"Room IDs: {sorted(rooms.keys())}")

    total_exits = sum(len(r.get("exits", [])) for r in rooms.values())
    print(f"Total exits: {total_exits}")

    if output_path:
        with open(output_path, "w") as f:
            json.dump(result, f, indent=2)
        print(f"Written to: {output_path}")
    else:
        print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
