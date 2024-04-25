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
