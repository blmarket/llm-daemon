#%%
import os
while not os.path.exists(".git"):
    os.chdir("..")
os.chdir("bihyung")
os.getcwd()

#%%
import sys
import random
import difflib
from pprint import pprint

module_path = os.path.abspath(os.path.join('./bihyung'))
if module_path not in sys.path:
    sys.path.append(module_path)

import maturin_import_hook
maturin_import_hook.install()
from bihyung import LlamaDaemon

#%%
d = LlamaDaemon()

#%%
d.fork_daemon()
d.heartbeat()

#%%
d.endpoint()

#%%
import requests

#%%
resp = requests.post(d.endpoint(), json = {
    "prompt": "<|begin_of_text|>Hello world",
    "n_predict": 128,
    "max_tokens": 128,
}).json()
resp
