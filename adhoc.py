#%%
# First cell is for the development.
import os
while not os.path.exists(".git"):
    os.chdir("..")
os.chdir("bihyung")
os.getcwd()

#%%
import sys

module_path = os.path.abspath(os.path.join('./bihyung'))
if module_path not in sys.path:
    sys.path.append(module_path)

import maturin_import_hook
maturin_import_hook.install()

#%%
import requests
from bihyung import Model, daemon_from_model

#%%
with daemon_from_model(Model.Gemma2b) as inner:
    print(inner)
    
#%%
inner = daemon_from_model(Model.Gemma2b)
inner.__enter__()

#%%
resp = requests.post(inner.endpoint() + "/completions", json = {
    "prompt": "<bos>Hello world",
    "n_predict": 128,
    "max_tokens": 128,
}).json()
resp["content"]

#%%
# Section: OpenAI demo
from openai import OpenAI

# %%
client = OpenAI(base_url = inner.endpoint())
resp = client.completions.create(model = "base", prompt = "<bos>Hello world", max_tokens = 128)
resp.content

# %%
resp = client.chat.completions.create(model = "base", messages = [{"role": "user", "content": "Hello world"}])
resp.choices[0].message.content

#%%
# After __exit__, we no longer use the daemon.
# Daemon will be kept alive for next 120 seconds, then close itself.
inner.__exit__()

# %%
