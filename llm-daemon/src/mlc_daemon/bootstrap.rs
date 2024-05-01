pub const PYPROJECT: &str = r#"[project]
name = "mlc-serv"
license = { text = "Proprietary" }
version = "0.1.0"
requires-python = ">= 3.9"
dependencies = [
    "fastapi",
    "mlc-llm-nightly-cu122 ; sys_platform != 'darwin'",
    "mlc-ai-nightly-cu122 ; sys_platform != 'darwin'",
    "mlc-llm-nightly ; sys_platform == 'darwin'",
    "mlc-ai-nightly ; sys_platform == 'darwin'",
]
"#;

pub fn script(python: &str) -> String {
    format!(
        r#"#!/bin/bash

export PYTHON={}
export VERSION=$PYTHON-0.3.0
export VENV_PATH=~/.cache/mlc-venv-$PYTHON
export APP_PATH=~/.cache/mlc-app-$PYTHON

if ! [[ -d $VENV_PATH ]]; then
    $PYTHON -m venv $VENV_PATH
fi

. $VENV_PATH/bin/activate

# check $APP_PATH and $APP_PATH/placeholder exists
if ! [[ -d $APP_PATH && -f $APP_PATH/placeholder && "$(cat $APP_PATH/placeholder)" = "$VERSION" ]]; then
    pip3 install -U -f 'https://mlc.ai/wheels' . --target $APP_PATH
	echo -n $VERSION > $APP_PATH/placeholder
fi

cd $APP_PATH

# Lesson learned: Use exec so parent can kill python
exec python -m mlc_llm serve $@
"#,
        python
    )
}
