#!/usr/bin/env python3
import json
import sys


def main() -> int:
    if "--health" in sys.argv:
        print("python-echo healthy")
        return 0
    if "--manifest" in sys.argv:
        print("python-echo manifest-ready")
        return 0
    if "--execute-json" in sys.argv:
        index = sys.argv.index("--execute-json")
        payload = json.loads(sys.argv[index + 1])
        text = payload.get("text") or ""
        print(
            json.dumps(
                {
                    "text": f"python-echo:{text}",
                    "data": {"echo": text},
                    "success": True,
                    "error": None,
                    "follow_ups": [],
                }
            )
        )
        return 0
    if "--execute-tool-json" in sys.argv:
        index = sys.argv.index("--execute-tool-json")
        payload = json.loads(sys.argv[index + 1])
        print(
            json.dumps(
                {
                    "success": True,
                    "data": {"echo": payload},
                    "error": None,
                    "execution_time_ms": 0,
                }
            )
        )
        return 0
    if "--execute-llm-json" in sys.argv:
        index = sys.argv.index("--execute-llm-json")
        payload = json.loads(sys.argv[index + 1])
        messages = payload.get("messages", [])
        text = ""
        if messages:
            text = messages[-1].get("content") or ""
        if "--stream" in sys.argv:
            chunks = [
                {"delta": f"python-echo:{text}", "finish_reason": None},
                {"delta": "", "finish_reason": "stop"},
            ]
            print(json.dumps(chunks))
            return 0
        print(
            json.dumps(
                {
                    "text": f"python-echo:{text}",
                    "usage": {
                        "prompt_tokens": 1,
                        "completion_tokens": 1,
                        "total_tokens": 2,
                    },
                    "finish_reason": "stop",
                }
            )
        )
        return 0
    if "--execute-agent-json" in sys.argv:
        index = sys.argv.index("--execute-agent-json")
        payload = json.loads(sys.argv[index + 1])
        action = payload.get("action")
        if action == "heartbeat":
            print(
                json.dumps(
                    {
                        "success": True,
                        "heartbeat": {
                            "success": True,
                            "completed_tasks": [],
                            "created_tasks": [],
                            "sent_messages": [],
                            "error": None,
                            "usage": {
                                "tokens": 0,
                                "cost_cents": 0,
                                "execution_time_ms": 1,
                            },
                        },
                        "loaded_skills": [],
                        "error": None,
                        "metadata": {},
                    }
                )
            )
            return 0
        if action in {"load_skill", "unload_skill", "shutdown"}:
            print(
                json.dumps(
                    {
                        "success": True,
                        "heartbeat": None,
                        "loaded_skills": [],
                        "error": None,
                        "metadata": {},
                    }
                )
            )
            return 0
        print(
            json.dumps(
                {
                    "success": False,
                    "heartbeat": None,
                    "loaded_skills": [],
                    "error": f"unknown action: {action}",
                    "metadata": {},
                }
            )
        )
        return 1
    print("python-echo plugin runtime")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
