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
