const { startDaemon } = require("./index.node");

console.log(require.resolve("./server"));

exports.startDaemon = startDaemon;