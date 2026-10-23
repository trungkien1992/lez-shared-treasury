"""Exercise real local HTTP framing, channel mismatches, and refusal to submit."""
import http.server
import json
import threading
import unittest
from unittest.mock import patch

import devnet_preflight as preflight


class PreflightTests(unittest.TestCase):
    def setUp(self):
        self.calls = []
        self.channel = "01" * 32
        self.responses = {
            "checkHealth": None, "getChannelId": self.channel, "getLastBlockId": 7,
            "getProgramIds": {}, "getProofsAndRoot": [[], "00" * 32],
        }
        outer = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                outer.calls.append(request)
                answer = {"jsonrpc": "2.0", "id": request["id"], "result": outer.responses[request["method"]]}
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps(answer).encode())

            def log_message(self, *args):
                pass

        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = f"http://127.0.0.1:{self.server.server_port}"

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def test_real_http_readonly_roundtrip(self):
        report = preflight.remote_checks(self.url, self.channel)
        self.assertTrue(report["channel_and_rpc_checks_passed"])
        self.assertFalse(report["runtime_binary_compatibility_verified"])
        self.assertEqual([x["method"] for x in self.calls],
                         ["checkHealth", "getChannelId", "getLastBlockId", "getProgramIds", "getProofsAndRoot"])
        self.assertEqual(self.calls[-1]["params"], [[]])

    def test_wrong_channel_stops_before_other_queries(self):
        with self.assertRaisesRegex(ValueError, "differs"):
            preflight.remote_checks(self.url, "02" * 32)
        self.assertEqual(len(self.calls), 2)

    def test_send_method_cannot_reach_network(self):
        with self.assertRaisesRegex(ValueError, "read-only"):
            preflight.rpc(self.url, "sendTransaction", [{}], 1)
        self.assertEqual(self.calls, [])

    def test_invalid_jsonrpc_response_is_failure(self):
        with patch.object(preflight, "rpc", return_value={"bad": "health"}):
            with self.assertRaisesRegex(ValueError, "health"):
                preflight.remote_checks(self.url, self.channel)

    def test_invalid_channel_is_failure_before_http(self):
        with self.assertRaisesRegex(ValueError, "32-byte"):
            preflight.remote_checks(self.url, "short")
        self.assertEqual(self.calls, [])

    def test_rpc_auth_in_url_is_not_logged_or_sent(self):
        with self.assertRaisesRegex(ValueError, "credentials"):
            preflight.remote_checks("https://example.invalid/?token=test", self.channel)
        self.assertEqual(self.calls, [])


if __name__ == "__main__":
    unittest.main()
