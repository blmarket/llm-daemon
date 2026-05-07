import os
import sys
import time
from pathlib import Path
from urllib.parse import urlparse

project_root = Path(__file__).parent.absolute()
sys.path.insert(0, str(project_root / "bihyung"))

import maturin_import_hook
from maturin_import_hook.settings import MaturinSettings

maturin_import_hook.install(
    settings=MaturinSettings(features=["cuda"]),
)

from bihyung import daemon_from_hf, _server_path

daemon = daemon_from_hf(
    # "bartowski/google_gemma-4-E2B-it-GGUF:Q4_K_M", # working
    # "bartowski/Qwen_Qwen3.6-27B-GGUF:Q4_K_M", # Gibberish
    "unsloth/Qwen3.6-27B-GGUF:UD-Q4_K_XL",
    _server_path,
    [ "--temp", "0.6", "--top-p", "0.95", "--top-k", "20", "--presence_penalty", "1.5", "--min-p", "0.00",
      "-ngl", "99", "-fa", "1", "-c", "131072",
      "--cache-type-k", "q4_0", "--cache-type-v", "q4_0" ],
)
daemon.__enter__()

endpoint = daemon.endpoint()
port = urlparse(endpoint).port
print(f"llama.cpp server endpoint: {endpoint}")
print(f"port: {port}", flush=True)

try:
    while True:
        time.sleep(3600)
except KeyboardInterrupt:
    daemon.__exit__(None, None, None)
