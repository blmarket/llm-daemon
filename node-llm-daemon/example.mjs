import { spawnDaemon } from "./index.js";

spawnDaemon("ggml-org/gpt-oss-20b-GGUF", [
  "--jinja", "-ngl", "99", "-fa", "--threads", "-1", "--ctx-size", "131072", "-ub", "2048", "-b", "2048"
]);
