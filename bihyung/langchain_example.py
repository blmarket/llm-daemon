#%%
import os
while not os.path.exists(".git"):
    os.chdir("..")
os.chdir("bihyung")
os.getcwd()

from bihyung import LlamaDaemon

#%%
d = LlamaDaemon()

#%%
d.fork_daemon()
d.heartbeat()

# %%
d.endpoint()

# %%
import langchain
from langchain_openai import ChatOpenAI

# %%
llm = ChatOpenAI(openai_api_base='http://127.0.0.1:28282/v1', openai_api_key='asdf')

# %%
llm.invoke("how can langsmith help with testing?")

# %%
