pub const PYPROJECT: &str = r#"[project]
name = "mlc-serv"
license = { text = "Proprietary" }
version = "0.1.0"
requires-python = ">= 3.11, < 3.12"
dependencies = [
    "fastapi",
    "mlc-llm-nightly-cu122 ; sys_platform != 'darwin'",
    "mlc-ai-nightly-cu122 ; sys_platform != 'darwin'",
    "mlc-llm-nightly ; sys_platform == 'darwin'",
    "mlc-ai-nightly ; sys_platform == 'darwin'",
]
"#;

pub const SCRIPT: &str = r#"#!/bin/bash

export PYTHON=python3.11
export APP_PATH=~/.cache/mlc-app-2
export VERSION='0.2.2'

# check $APP_PATH and $APP_PATH/placeholder exists
if ! [[ -d $APP_PATH && -f $APP_PATH/placeholder && "$(cat $APP_PATH/placeholder)" = "$VERSION" ]]; then
	# Big headache - what if user does not have pip?
	$PYTHON -m pip install -f 'https://mlc.ai/wheels' . --target $APP_PATH
	echo -n $VERSION > $APP_PATH/placeholder
fi

cd $APP_PATH

# Lesson learned: Use exec so parent can kill python
exec $PYTHON -m mlc_llm serve $@
"#;
