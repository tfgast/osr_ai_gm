#!/usr/bin/env python3
"""Pipeline server: integrated map extraction UI.

Starts a local HTTP server that serves the review UI in pipeline mode,
providing REST API endpoints for step execution, state management,
and file serving.

Usage:
    python pipeline_server.py --labeled-map MAP.png \
        [--unlabeled-map MAP.png] [--module-text module.json] \
        [--model opus] [--output-dir ./output] [--port 8000]

To resume a previous run, re-run from the same --output-dir.
"""

import argparse
import json
import os
import subprocess
import sys
import threading
import time
import webbrowser
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from urllib.parse import parse_qs, urlparse

SCRIPT_DIR = Path(__file__).parent.resolve()

MODELS = ["opus", "sonnet", "haiku", "gemini-2.5-pro", "gemini-2.5-flash"]

# Step definitions: id, display name, requires_review
STEP_DEFS = [
    ("step0", "Text Extract"),
    ("step1_diff", "Diff Labels"),
    ("step1_features", "Features"),
    ("step2", "Locations"),
    ("annotate", "Annotate Map"),
    ("step3", "Connections"),
    ("step4", "Descriptions"),
]

# Steps that get a review phase
REVIEW_STEPS = {"step1_features", "step2", "step3", "step4"}

# Steps that auto-advance (no review needed)
AUTO_STEPS = {"annotate"}


class PipelineState:
    """Thread-safe pipeline state with JSON persistence."""

    def __init__(self, state_path: Path, config: dict):
        self.state_path = state_path
        self.lock = threading.Lock()

        if state_path.exists():
            with open(state_path) as f:
                self.state = json.load(f)
            # Update input paths from CLI args (may have changed)
            self.state["config"]["inputs"] = config["inputs"]
            print(f"  Resumed pipeline from {state_path}")
        else:
            self.state = {
                "config": config,
                "steps": {},
                "current_step": None,
                "log": "",
            }
            self._init_steps(config)
            print(f"  New pipeline state created")

        self._save()

    def _init_steps(self, config):
        """Initialize step statuses based on available inputs."""
        inputs = config["inputs"]
        has_module = bool(inputs.get("module_text"))
        has_unlabeled = bool(inputs.get("unlabeled_map"))

        for step_id, display_name in STEP_DEFS:
            status = "pending"
            if step_id == "step0" and not has_module:
                status = "unavailable"
            elif step_id == "step1_diff" and not has_unlabeled:
                status = "unavailable"

            self.state["steps"][step_id] = {
                "id": step_id,
                "name": display_name,
                "status": status,
                "error": None,
                "started_at": None,
                "finished_at": None,
                "output_file": None,
            }

    def _save(self):
        """Write state atomically."""
        tmp = self.state_path.with_suffix(".tmp")
        with open(tmp, "w") as f:
            json.dump(self.state, f, indent=2)
        tmp.replace(self.state_path)

    def get(self) -> dict:
        with self.lock:
            return json.loads(json.dumps(self.state))

    def set_step_status(self, step_id: str, status: str, error: str = None,
                        output_file: str = None):
        with self.lock:
            step = self.state["steps"][step_id]
            step["status"] = status
            step["error"] = error
            if status == "running":
                step["started_at"] = time.strftime("%Y-%m-%dT%H:%M:%S")
            if status in ("review", "approved", "error"):
                step["finished_at"] = time.strftime("%Y-%m-%dT%H:%M:%S")
            if output_file:
                step["output_file"] = output_file
            self._save()

    def dirty_steps_after(self, step_id: str):
        """Reset all steps after step_id back to pending."""
        with self.lock:
            step_ids = [s[0] for s in STEP_DEFS]
            try:
                idx = step_ids.index(step_id)
            except ValueError:
                return
            for later_id in step_ids[idx + 1:]:
                step = self.state["steps"][later_id]
                if step["status"] in ("approved", "review", "error"):
                    step["status"] = "pending"
                    step["error"] = None
            self._save()

    def set_log(self, text: str):
        with self.lock:
            self.state["log"] = text
            # Don't save on every log update — too frequent

    def get_log(self) -> str:
        with self.lock:
            return self.state["log"]

    def update_config(self, new_config: dict):
        with self.lock:
            self.state["config"].update(new_config)
            self._save()

    def get_next_step(self) -> str | None:
        """Find the first pending step."""
        with self.lock:
            for step_id, _ in STEP_DEFS:
                s = self.state["steps"][step_id]["status"]
                if s == "pending":
                    return step_id
            return None

    def is_running(self) -> bool:
        with self.lock:
            return any(
                s["status"] == "running"
                for s in self.state["steps"].values()
            )


class PipelineRunner:
    """Builds and runs step commands in background threads."""

    def __init__(self, state: PipelineState, output_dir: Path):
        self.state = state
        self.output_dir = output_dir
        self.process: subprocess.Popen | None = None
        self.thread: threading.Thread | None = None

    def run_step(self, step_id: str):
        """Start a step in a background thread."""
        if self.state.is_running():
            raise RuntimeError("A step is already running")

        self.state.set_log("")
        self.thread = threading.Thread(
            target=self._execute, args=(step_id,), daemon=True
        )
        self.thread.start()

    def _execute(self, step_id: str):
        """Execute a single step (runs in background thread)."""
        self.state.set_step_status(step_id, "running")
        config = self.state.get()["config"]
        inputs = config["inputs"]
        model = config.get("models", {}).get(step_id, config.get("default_model", "opus"))

        try:
            cmd = self._build_command(step_id, inputs, model, config)
            if cmd is None:
                self.state.set_step_status(step_id, "error",
                                           error=f"Cannot build command for {step_id}")
                return

            self.state.set_log(f"$ {' '.join(cmd)}\n\n")
            self.process = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                cwd=str(self.output_dir),
            )

            log_lines = []
            for line in self.process.stdout:
                log_lines.append(line)
                self.state.set_log("$ " + " ".join(cmd) + "\n\n" + "".join(log_lines))

            self.process.wait()
            if self.process.returncode != 0:
                self.state.set_step_status(
                    step_id, "error",
                    error=f"Process exited with code {self.process.returncode}",
                )
                return

            # Determine output file
            output_file = self._get_output_file(step_id)

            if step_id in AUTO_STEPS:
                self.state.set_step_status(step_id, "approved", output_file=output_file)
                # Auto-advance to next step
                next_step = self.state.get_next_step()
                if next_step:
                    self._execute(next_step)
            elif step_id in REVIEW_STEPS:
                self.state.set_step_status(step_id, "review", output_file=output_file)
            else:
                self.state.set_step_status(step_id, "approved", output_file=output_file)

        except Exception as e:
            self.state.set_step_status(step_id, "error", error=str(e))
        finally:
            self.process = None

    def _build_command(self, step_id: str, inputs: dict, model: str,
                       config: dict) -> list[str] | None:
        """Build the subprocess command for a step."""
        py = sys.executable
        od = self.output_dir

        labeled = inputs.get("labeled_map", "")
        unlabeled = inputs.get("unlabeled_map", "")
        module_text = inputs.get("module_text", "")

        step0_out = str(od / "step0_output.json")
        step1_labels = str(od / "step1_labels.json")
        step1_out = str(od / "step1_output.json")
        step1_reviewed = str(od / "step1_output_reviewed.json")
        step2_out = str(od / "step2_output.json")
        step2_reviewed = str(od / "step2_output_reviewed.json")
        annotated_map = str(od / "annotated_map.png")
        step3_out = str(od / "step3_output.json")
        step3_reviewed = str(od / "step3_output_reviewed.json")
        step4_out = str(od / "step4_output.json")

        if step_id == "step0":
            if not module_text:
                return None
            return [py, str(SCRIPT_DIR / "step0_text_extract.py"),
                    module_text, step0_out, "--model", model]

        elif step_id == "step1_diff":
            if not unlabeled:
                return None
            cmd = [py, str(SCRIPT_DIR / "step1_diff_labels.py"),
                   labeled, unlabeled, step1_labels, "--model", model]
            if Path(step0_out).exists():
                cmd += ["--step0", step0_out]
            return cmd

        elif step_id == "step1_features":
            cmd = [py, str(SCRIPT_DIR / "step1_features.py"),
                   labeled, step1_out, "--model", model]
            if Path(step0_out).exists():
                cmd += ["--step0", step0_out]
            if Path(step1_labels).exists():
                cmd += ["--labels", step1_labels]
            return cmd

        elif step_id == "step2":
            return [py, str(SCRIPT_DIR / "step2_locations.py"),
                    labeled, step1_reviewed, step2_out, "--model", model]

        elif step_id == "annotate":
            return [py, str(SCRIPT_DIR / "annotate_map.py"),
                    labeled, step2_reviewed, annotated_map]

        elif step_id == "step3":
            map_img = unlabeled or labeled
            cmd = [py, str(SCRIPT_DIR / "step3_connections.py"),
                   map_img, step1_reviewed, step2_reviewed, step3_out,
                   "--debug-dir", str(od)]
            s3p = config.get("step3_params", {})
            if s3p.get("threshold"):
                cmd += ["--threshold", str(s3p["threshold"])]
            if s3p.get("kernel"):
                cmd += ["--kernel", str(s3p["kernel"])]
            return cmd

        elif step_id == "step4":
            cmd = [py, str(SCRIPT_DIR / "step4_descriptions.py"),
                   step3_reviewed, step4_out, "--model", model]
            if module_text:
                cmd += ["--module-text", module_text]
            if Path(step2_reviewed).exists():
                cmd += ["--step2-json", step2_reviewed]
            return cmd

        return None

    def _get_output_file(self, step_id: str) -> str:
        """Return the logical output filename for a step."""
        mapping = {
            "step0": "step0_output.json",
            "step1_diff": "step1_labels.json",
            "step1_features": "step1_output.json",
            "step2": "step2_output.json",
            "annotate": "annotated_map.png",
            "step3": "step3_output.json",
            "step4": "step4_output.json",
        }
        return mapping.get(step_id, "")


# --- File mapping for /api/file/<name> ---

def get_file_map(output_dir: Path, inputs: dict) -> dict[str, Path | None]:
    """Map logical file names to actual paths."""
    od = output_dir
    labeled = inputs.get("labeled_map")
    unlabeled = inputs.get("unlabeled_map")
    module_text = inputs.get("module_text")
    m = {
        "labeled_map": Path(labeled) if labeled else None,
        "unlabeled_map": Path(unlabeled) if unlabeled else None,
        "module_text": Path(module_text) if module_text else None,
        "step0_output": od / "step0_output.json",
        "step1_labels": od / "step1_labels.json",
        "step1_output": od / "step1_output.json",
        "step1_output_reviewed": od / "step1_output_reviewed.json",
        "step2_output": od / "step2_output.json",
        "step2_output_reviewed": od / "step2_output_reviewed.json",
        "annotated_map": od / "annotated_map.png",
        "step3_debug_binary": od / "step3_debug_binary.png",
        "step3_debug_watershed": od / "step3_debug_watershed.png",
        "step3_output": od / "step3_output.json",
        "step3_output_reviewed": od / "step3_output_reviewed.json",
        "step4_output": od / "step4_output.json",
    }
    return m


# Reviewed-file fallback: maps raw name -> reviewed name
REVIEWED_VARIANTS = {
    "step1_output": "step1_output_reviewed",
    "step2_output": "step2_output_reviewed",
    "step3_output": "step3_output_reviewed",
}


# --- HTTP Handler ---

class PipelineHandler(SimpleHTTPRequestHandler):
    """HTTP handler with pipeline REST API."""

    pipeline_state: PipelineState = None
    pipeline_runner: PipelineRunner = None
    output_dir: Path = None
    inputs: dict = None

    def log_message(self, format, *args):
        """Suppress default access logs."""
        pass

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        if path == "/review.html" or path == "" or path == "/":
            self._serve_review_html()
        elif path == "/api/state":
            self._json_response(self.pipeline_state.get())
        elif path == "/api/log":
            self._json_response({"log": self.pipeline_state.get_log()})
        elif path.startswith("/api/file/"):
            name = path[len("/api/file/"):]
            self._serve_file(name)
        else:
            self.send_error(404)

    def do_POST(self):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        body = self._read_body()

        if path == "/api/configure":
            self._handle_configure(body)
        elif path.startswith("/api/run/"):
            step_id = path[len("/api/run/"):]
            self._handle_run(step_id)
        elif path == "/api/save":
            self._handle_save(body)
        elif path.startswith("/api/approve/"):
            step_id = path[len("/api/approve/"):]
            self._handle_approve(step_id, body)
        else:
            self.send_error(404)

    def _read_body(self) -> dict:
        length = int(self.headers.get("Content-Length", 0))
        if length == 0:
            return {}
        raw = self.rfile.read(length)
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return {}

    def _json_response(self, obj: dict, status: int = 200):
        data = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(data)

    def _serve_review_html(self):
        html_path = SCRIPT_DIR / "review.html"
        if not html_path.exists():
            self.send_error(404, "review.html not found")
            return
        data = html_path.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _serve_file(self, name: str):
        parsed = urlparse(self.path)
        qs = parse_qs(parsed.query)
        prefer_reviewed = "prefer_reviewed" in qs

        file_map = get_file_map(self.output_dir, self.inputs)

        # If prefer_reviewed, try the reviewed variant first
        if prefer_reviewed and name in REVIEWED_VARIANTS:
            reviewed_name = REVIEWED_VARIANTS[name]
            reviewed_path = file_map.get(reviewed_name)
            if reviewed_path and reviewed_path.exists() and reviewed_path.is_file():
                name = reviewed_name

        if name not in file_map:
            self.send_error(404, f"Unknown file: {name}")
            return

        fpath = file_map[name]
        if fpath is None or not fpath.exists() or not fpath.is_file():
            self.send_error(404, f"File not found: {name}")
            return

        # Determine content type
        suffix = fpath.suffix.lower()
        ct = {
            ".json": "application/json",
            ".png": "image/png",
            ".jpg": "image/jpeg",
            ".jpeg": "image/jpeg",
        }.get(suffix, "application/octet-stream")

        data = fpath.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", ct)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(data)

    def _handle_configure(self, body: dict):
        updates = {}
        if "default_model" in body:
            updates["default_model"] = body["default_model"]
        if "models" in body:
            updates["models"] = body["models"]
        if "step3_params" in body:
            updates["step3_params"] = body["step3_params"]
        if updates:
            self.pipeline_state.update_config(updates)
        self._json_response({"ok": True})

    def _handle_run(self, step_id: str):
        state = self.pipeline_state.get()
        if step_id not in state["steps"]:
            self._json_response({"error": f"Unknown step: {step_id}"}, 400)
            return

        step = state["steps"][step_id]
        allowed = ("pending", "error", "review", "approved")
        if step["status"] not in allowed:
            self._json_response(
                {"error": f"Step {step_id} is {step['status']}, cannot run"}, 400
            )
            return

        if self.pipeline_state.is_running():
            self._json_response({"error": "A step is already running"}, 409)
            return

        # Re-running a completed step dirties all later steps
        if step["status"] in ("approved", "review"):
            self.pipeline_state.dirty_steps_after(step_id)

        try:
            self.pipeline_runner.run_step(step_id)
            self._json_response({"ok": True, "step": step_id})
        except RuntimeError as e:
            self._json_response({"error": str(e)}, 409)

    def _handle_save(self, body: dict):
        """Save user edits to a step's output file."""
        step_id = body.get("step_id")
        data = body.get("data")
        conn_data = body.get("connData")

        if not step_id:
            self._json_response({"error": "step_id required"}, 400)
            return

        # Determine which file to save
        reviewed_file = self._get_reviewed_path(step_id)
        if not reviewed_file:
            self._json_response({"error": f"No output file for {step_id}"}, 400)
            return

        if step_id == "step3" and conn_data is not None:
            # Step 3: save connection data to step3_reviewed
            with open(reviewed_file, "w") as f:
                json.dump(conn_data, f, indent=2)
            # Also persist location edits back to step2_reviewed
            if data is not None:
                step2_reviewed = self.output_dir / "step2_output_reviewed.json"
                with open(step2_reviewed, "w") as f:
                    json.dump(data, f, indent=2)
        elif data is not None:
            with open(reviewed_file, "w") as f:
                json.dump(data, f, indent=2)
        else:
            self._json_response({"error": "No data to save"}, 400)
            return

        self._json_response({"ok": True, "saved": str(reviewed_file)})

    def _handle_approve(self, step_id: str, body: dict):
        """Save edits and mark step as approved."""
        state = self.pipeline_state.get()
        if step_id not in state["steps"]:
            self._json_response({"error": f"Unknown step: {step_id}"}, 400)
            return

        step = state["steps"][step_id]
        if step["status"] != "review":
            self._json_response(
                {"error": f"Step {step_id} is {step['status']}, cannot approve"}, 400
            )
            return

        # Save the data first
        data = body.get("data")
        conn_data = body.get("connData")
        reviewed_file = self._get_reviewed_path(step_id)

        if reviewed_file:
            if step_id == "step3" and conn_data is not None:
                with open(reviewed_file, "w") as f:
                    json.dump(conn_data, f, indent=2)
                # Also persist location edits back to step2_reviewed
                if data is not None:
                    step2_reviewed = self.output_dir / "step2_output_reviewed.json"
                    with open(step2_reviewed, "w") as f:
                        json.dump(data, f, indent=2)
            elif data is not None:
                with open(reviewed_file, "w") as f:
                    json.dump(data, f, indent=2)

        self.pipeline_state.set_step_status(step_id, "approved")

        # Auto-run next pending step
        next_step = self.pipeline_state.get_next_step()
        response = {"ok": True, "approved": step_id, "next": next_step}

        if next_step:
            try:
                self.pipeline_runner.run_step(next_step)
                response["auto_running"] = next_step
            except RuntimeError:
                pass

        self._json_response(response)

    def _get_reviewed_path(self, step_id: str) -> Path | None:
        """Get the reviewed output path for a step."""
        od = self.output_dir
        mapping = {
            "step1_features": od / "step1_output_reviewed.json",
            "step2": od / "step2_output_reviewed.json",
            "step3": od / "step3_output_reviewed.json",
            "step4": od / "step4_output.json",  # step4 doesn't have separate reviewed
        }
        return mapping.get(step_id)

    def do_OPTIONS(self):
        """Handle CORS preflight."""
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()


def main():
    parser = argparse.ArgumentParser(
        description="Pipeline server for map extraction",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--labeled-map", required=True,
                        help="Path to labeled map image")
    parser.add_argument("--unlabeled-map", default=None,
                        help="Path to unlabeled map image (optional)")
    parser.add_argument("--module-text", default=None,
                        help="Path to module text JSON (optional)")
    parser.add_argument("--model", default="opus",
                        help="Default LLM model (default: opus)")
    parser.add_argument("--output-dir", default="./output",
                        help="Output directory (default: ./output)")
    parser.add_argument("--port", type=int, default=8000,
                        help="Server port (default: 8000)")
    args = parser.parse_args()

    # Validate inputs
    labeled = Path(args.labeled_map).resolve()
    if not labeled.exists():
        print(f"Error: labeled map not found: {labeled}", file=sys.stderr)
        sys.exit(1)

    unlabeled = None
    if args.unlabeled_map:
        unlabeled = Path(args.unlabeled_map).resolve()
        if not unlabeled.exists():
            print(f"Error: unlabeled map not found: {unlabeled}", file=sys.stderr)
            sys.exit(1)

    module_text = None
    if args.module_text:
        module_text = Path(args.module_text).resolve()
        if not module_text.exists():
            print(f"Error: module text not found: {module_text}", file=sys.stderr)
            sys.exit(1)

    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    inputs = {
        "labeled_map": str(labeled),
        "unlabeled_map": str(unlabeled) if unlabeled else "",
        "module_text": str(module_text) if module_text else "",
    }

    config = {
        "inputs": inputs,
        "default_model": args.model,
        "models": {},
    }

    state_path = output_dir / "pipeline_state.json"
    pipeline_state = PipelineState(state_path, config)
    runner = PipelineRunner(pipeline_state, output_dir)

    # Configure handler class
    PipelineHandler.pipeline_state = pipeline_state
    PipelineHandler.pipeline_runner = runner
    PipelineHandler.output_dir = output_dir
    PipelineHandler.inputs = inputs

    server = HTTPServer(("localhost", args.port), PipelineHandler)
    url = f"http://localhost:{args.port}/review.html?pipeline"

    print(f"\n  Pipeline server running at {url}")
    print(f"  Output dir: {output_dir}")
    print(f"  Press Ctrl+C to stop\n")

    # Open browser
    webbrowser.open(url)

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n  Shutting down...")
        server.shutdown()


if __name__ == "__main__":
    main()
