# @blmarket/llm-daemon

Creates LLM daemon only when it's necessary.

It creates a llama.cpp server instance, leave it running while the application
is running.

It will stop the server only if there's no client using it for 30 seconds.

## Caveat

Note that this package includes llama.cpp server binary for Linux-x86_64 with
GNU libc, with CUDA enabled. It's unlikely to work on other platforms.