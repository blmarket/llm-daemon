## Changes on 0.7.0 - javascript runtime support

https://www.npmjs.com/package/@blmarket/llm-daemon

Note that version is not central - had to use 0.7.1 for npm version, later we will consolidate this.

Also deprecated bunch of things.

---

## Changes on 0.6.3 - Daemon logging had a bug

I found the log files are not being updated until a point the new log contents
accumulate until a point.

It was my bad that I chose the wrong option to open a log file. Either it should
enable append or truncate, but chose none of them was a bad decision.

Learned it in a hard way...

---

## Let's create an example where proxy is working as expected

At this point it's hard to tell whether the proxy is really worth. Would be
nice to have an example to showcase the proxy.

Goal: Run server with/without proxy. run multiple requests, and show their
process order and time.

---

## Separate proxy to be a feature

Even though it's a useless yak shaving, I found proxy to be an optional feature
of the crate.

May need to draft some nice description of proxy feature...

Belos is the draft of the description.

### Last-In-First-Out proxy to the llama.cpp server

Problem: When code generation requests are congested, the usual server will
respond to the requests in the order they are received.

Although this can work for most of the cases, there are cases where the old
request is no longer relevant, because the user updated their prompt to a newer
version. When server has multiple pending requests, it's the best to process the
newest request first.

This proxy address that concern by properly hold the request connection, and
discard the request when the request connection is dropped by the client.

---

## Fix proxy to work well with latest llama.cpp

Previously it was possible to make health check by `GET /completions`, but it
seems more recent version responds with 405 method not allowed error for GET
request (it should be POST obviously)

Fixed it by replacing the health check endpoint of the proxy to be `GET /health`

---

## Better release with cargo workspaces

```
cargo workspaces version --force '*'
git push github main vx.y.z
```

요렇게 하면 모든 프로젝트의 버전을 한 번에 올릴 수 있습니다. 하는김에 crates.io
publish도 Github action으로 자동화해 놨으니 이제 버전업만 하고 tag만 push하면 되지
않을까요?

---

## 0.3.14가 되어서야 정리하는 버전 업데이트 방법

보통 llama.cpp가 업데이트되어있을테니 그거 업데이트 먼저 하고 오세요

```
cargo workspaces version --no-git-push --no-individual-tags --force '*'
git push github main vx.y.z
```

---

## 0.3.0 feature - embed llama.cpp binary

지금은 그냥 llama.cpp를 직접 컴파일한 바이너리 경로를 지정하게 해 뒀는데, 기왕
daemon 패키지까지 만든 마당에 아예 바이너리를 함께 배포하는건 어떨까 싶다.

기본 cmake로 간단히 빌드를 해 봤고 rust 빌드에선 아마 잘 동작할 거다.

문제는 python 패키지인데, 여기 패키지에 server 바이너리랑 metal 파일을 리소스로
포함시켜야 할 것 같다. 요거까지 하고 0.3.0으로 판올림하면 적당할듯

---

## bootstrapping timeout of llm-daemon

알지 못할 이유로 초기 데몬 bootstrap 중에 heartbeat를 받지 못하고 timeout이
발생하고 있다. 맥북에서 발생하는 걸 발견했는데 리눅스에서도 동일한 문제가
있는지는 잘 모르겠다.

---

## Performance issue with the LLM models

llamafile Llama-3-8B 모델로 모두 대통합하려고 했는데, 막상 해보니까 내가 쓰려는
곳에서 생각보다 너무 느리다. 미워도 다시한번 mlc-llm으로 다시 시도해봐야 하나?
전반적으로 내가 쓰려는 요구조건에 맞춰서 벤치마크를 만들고 여러 모델에서
검증해볼 필요가 있겠다는 생각이 든다.

- 서버 application
- 모델
- size 및 quantization
- 테스트 케이스
  - 하나의 테스트 케이스에 여러 개의 checkpoint가 있으면 보기 좋을 것 같다.
- 시간 (millisec 단위)

---

## Unfixable timing bug

방금 유닛 테스트 돌리다가 발견한 버그인데, 아마 못고칠거야.

데몬이 죽는 시점에 정확히 맞춰서 새 데몬을 띄우면, 새 데몬은 아직 lock 파일
있으니까 자살하고, 기존 데몬은 새 클라이언트가 아직 달라붙지 않아서 그냥
죽어버리고, 클라이언트가 데몬이 살아있는 줄 알고 붙으려고 하면 실패...

데몬 timeout을 더 길게 잡으면 발생가능성이 0에 수렴할 테지만, 바꿀지 말지 아직은
좀 아리까리...

---

## Bug - inconsistent failure when port is already in use

./Meta-Llama-3-8B-Instruct.Q5_K_M.llamafile -ngl 9999 --port 28282 --nobrowser

이렇게 서버 띄우고 proxy.rs에 있는 테스트를 실행하면 절반 정도? 에러가 발생함.

---

## Need refactoring

원래 이런저런 로컬 서버를 모두 지원하고 싶어서 llama.cpp와 mlc-llm을 대상으로
두 개의 daemon 구현체를 만들어봤었는데, 만들고 보니까 비슷비슷한 코드가 두 벌로
중복되는 것 같다. 합칠 수 있는 부분은 합치는 것이 좋을 것 같다. 예를 들자면:

- 실제 server를 감시하면서 heartbeat를 받는 부분
  - tokio의 current_thread_runtime을 생성하는 부분
  - 해당 runtime에서 select!로 이런저런 이벤트를 처리하는 부분

---

## Release 0.1.2

`/tmp/llama-daemon.stderr`에 데몬 로그가 추가되었습니다. 기존에는 서버의 로그만
있었는데, 이젠 서버를 관리하는 데몬의 로그도 섞어찌개로 보이게 됩니다. 필요하면
로그 파일을 분리해야겠죠.

---

## Release process

좀 번거롭긴 한데... 현재는 이렇게 하고 있습니다:

1. workspace 내 project들의 Cargo.toml에서 version을 업데이트합니다.
2. 업데이트 커밋을 생성하고 `git tag 0.1.0`과 같이 태그를 만듭니다.
3. `llm-daemon`은 `cargo publish`로 새 버전을 배포합니다.
4. `bihyung`의 경우 Github workflow를 이용합니다. 그냥 tag를 push하면 배포가
   됩니다.  
   `git push github 0.1.0`

---

## Python에서 fork_daemon을 호출하는 것이 조금 이상해보인다.

이미 instance를 만들었는데 거기서 굳이 또 fork_daemon을 할 필요가 있나? 그냥
생성 시점에 알아서 fork를 시도하는 편이 어떨까 싶다. 원래라면 Context protocol을
써서 `__enter__` / `__exit__`를 활용하는 편이 더 좋을 것 같긴 한데, pyo3을 써서
그걸 구현할 수 있을지 조금 고민이다.

---

## Consolidating other llm apps to use llm-daemon

I'd like to apply it in my production usages and see it's working as expected.
Also moving all my existing models to Meta Llama 3-8b-it, hoping it can be the
best for my use cases.

I'm missing drop support proxy feature though.

---

## 비형

Local LLM, which 

---

## Object lifecycle needs some cleanup

Bunch of legacy object instantiations are messing with the new trait definition.

Remove all `::new()` methods and force them to use `::spawn_daemon()` instead.

---

## 파이썬 환경 제공

Interactive environment is a good place to demonstrate where the daemon approach
can be useful. It allows multiple python interactive environments to run the LLM
without loading the model multiple times.

Python will allow us to demonstrate the daemon is running while user is using
jupyter, and automatically closed when there is no activity.

---
