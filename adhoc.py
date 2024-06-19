#%%
# First cell is for the development.
import os
while not os.path.exists(".git"):
    os.chdir("..")
os.chdir("bihyung")
os.getcwd()

#%%
import sys
from openai import OpenAI

module_path = os.path.abspath(os.path.join('./bihyung'))
if module_path not in sys.path:
    sys.path.append(module_path)

import maturin_import_hook
maturin_import_hook.install()

#%%
import requests
from bihyung import Model, daemon_from_model, daemon_from_gguf

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
inner = daemon_from_gguf("/home/blmarket/proj/Meta-Llama-3-8B-Instruct-Q5_K_M.gguf")
inner.__enter__()

#%%
inner = daemon_from_model(Model.Llama3_8b)
inner.__enter__()

#%%
inner.endpoint()

# %%
client = OpenAI(base_url = inner.endpoint(), api_key = "nothing")

#%%
client.chat.completions.create(model = "base", messages = [
    {"role": "system", "content": """You are a coding companion.
You need to suggest code for the language python
Given some code prefix and suffix for context, output code should follow the prefix code.
You should only output valid code in the language python. 
To clearly define a code block, including white space, we will wrap the code block with tags.
Make sure to respect the white space and indentation rules of the language.
Do not output anything in plain language, make sure you only use the relevant programming language verbatim.
For example, consider the following request:
<begin_code_prefix>def print_hello():<end_code_prefix><begin_code_suffix>\n    return<end_code_suffix><begin_code_middle>
Your answer should be:

    print("Hello")<end_code_middle>     
"""},
    {"role": "user", "content": """<begin_code_prefix>def print_hello():<end_code_prefix><begin_code_suffix>\n    return<end_code_suffix><begin_code_middle>"""}
])
#%%
