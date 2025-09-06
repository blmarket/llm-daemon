const { startDaemon } = require("./index.node");

const serverPath = require.resolve("./server");

exports.startDaemon = (hf_repo, args) => {
    console.log(serverPath);
    return startDaemon(hf_repo, args, serverPath);
}