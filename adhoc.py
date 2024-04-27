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
# Test with Gen2 later
# gen = Generator("HF://mlc-ai/gemma-2b-it-q4f16_1-MLC")
gen = Gen2("HF://mlc-ai/gemma-2b-it-q4f16_1-MLC")
bert = BertEngine()

#%%
path = "./sample-docs/en.md"
doc_text = None
with open(path, "r") as fp:
    doc_text = fp.read()
doc = Doktor(doc_text)
first_50_sections = doc.sections()[:50]
p = NoteChatPrompt(gen, first_50_sections)
# p.user("Using the contents of the note, can you explain what is lsp-md? Please use the same language as the note. Just use 2~3 sentences to explain, without using bullet points.")
p.user("Using the contents of the note, can you explain what is daemonize? Please use the same language as the note. Just use 2~3 sentences to explain, without using bullet points.")
print(p.ask())

#%%
print("".join([s.all() for s in first_50_sections]))

#%%
path = "./sample-docs/note.md"
doc_text = None
with open(path, "r") as fp:
    doc_text = fp.read()
doc = Doktor(doc_text)

#%%
section = random.choice(doc.sections())
print(section.all())

#%%
first_50_sections = doc.sections()[:50]
p = NoteChatPrompt(gen, first_50_sections)
p.user("Using the contents of the note, can you explain what is lsp-md? Please use the same language as the note.")
print(p.ask())

#%%
print("".join([s.all() for s in first_50_sections]))


#%%
bert.extract_keywords(section)

# %%
p = NoteChatPrompt(gen, section)
p.user("What is the note about?")
print(p.ask())
p.user("Can you suggest a better title for the note?")
print(p.ask("Sure, here is a suggested title: **", stop=["**"]))

#%%
print(p.prompt())

# %%
p = NoteChatPrompt(gen, section)
p.user("What is the notable URL the note mentions? If none exists, say `None`")
url = p.ask("The notable URL is:\n\n`", stop=["`"])
url

# %%
p = ChatPrompt()
p.user(f"Given the url: `{url}`, what is the keywords worth to take a look? Give the answer in the list format.")
gen.gen(p.ask("The keywords in the url are:\n\n"))

# %%
p = NoteChatPrompt(gen, section)
p.user("As a professional proofreader, criticize the note for possible improvements focusing on the overall clarity.")
critic = p.ask("")
print(critic)

# %%
p = NoteChatPrompt(gen, section)
p.user("As a professional proofreader, try to rewrite the note with corrected grammar and punctuation. Do not try to modify content wrapped by backticks.")
corrected = p.ask("Sure, here is the updated note:\n\n")
t1 = section.all().splitlines()
t2 = corrected.splitlines()
pprint(list(difflib.Differ().compare(t1, t2)))

# %%

#%%
p = RawPrompt()
p.add_system_prompt("""<bos>You are answering a next number of the sequence.
<start_of_turn>user
1 2 3 4 5<end_of_turn>
<start_of_turn>model
""")
gen.gen(str(p))

# %%
pb = ChatPrompt("You are answering a next number of the sequence.")
pb.user("1 2 3 4 5")
pb.model("6")
pb.user("7 8 9")
pb.model("10")
pb.user("11 12")
gen.gen(pb.ask("13 14"))
