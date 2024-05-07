#%%
import os
while not os.path.exists(".git"):
    os.chdir("..")
os.chdir("bihyung")
os.getcwd()

#%%
# Temporary: I'm using custom build maturin, so can't use import_hook
from bihyung import Model, daemon_from_model, server_path

#%%
import sys

module_path = os.path.abspath(os.path.join('./bihyung'))
if module_path not in sys.path:
    sys.path.append(module_path)

import maturin_import_hook
maturin_import_hook.install()
from bihyung import Model, daemon_from_model, server_path

#%%
d = daemon_from_model(Model.Gemma2b, server_path)

#%%
d.fork_daemon()
d.heartbeat()

#%%
d.endpoint()

#%%
import requests

#%%
resp = requests.post(d.endpoint() + "/completions", json = {
    "prompt": "<bos>Hello world",
    "n_predict": 128,
    "max_tokens": 128,
}).json()
resp["content"]

# %%
resp

# %%
