pub const PYPROJECT: &str = r#"[project]
name = "mlc-serv"
license = { text = "Proprietary" }
version = "0.1.0"
requires-python = ">= 3.9, <= 3.12"
dependencies = [
    "fastapi",
    "mlc-llm-nightly-cu122 ; sys_platform != 'darwin'",
    "mlc-ai-nightly-cu122 ; sys_platform != 'darwin'",
    "mlc-llm-nightly ; sys_platform == 'darwin'",
    "mlc-ai-nightly ; sys_platform == 'darwin'",
]
"#;

pub fn script(python: &str) -> String {
    format!(r#"#!/bin/bash

export PYTHON={}
export VERSION='{}-0.2.4'
export VENV_PATH=~/.cache/mlc-venv-$PYTHON
export APP_PATH=~/.cache/mlc-app

# check $APP_PATH and $APP_PATH/placeholder exists
if ! [[ -d $APP_PATH && -f $APP_PATH/placeholder && "$(cat $APP_PATH/placeholder)" = "$VERSION" ]]; then
    if ! [[ -d $VENV_PATH ]]; then
        $PYTHON -m venv $VENV_PATH
    fi
	~/.cache/mlc-venv-$PYTHON/bin/pip3 install -U -f 'https://mlc.ai/wheels' . --target $APP_PATH
	echo -n $VERSION > $APP_PATH/placeholder
fi

cd $APP_PATH

# Lesson learned: Use exec so parent can kill python
exec $PYTHON -m mlc_llm serve $@
"#, python, python)
}
