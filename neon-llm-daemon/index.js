const { startDaemon, ping } = require("./index.node");

const serverPath = require.resolve("./server");

exports.startDaemon = (hf_repo, args) => {
    const daemon = startDaemon(serverPath, hf_repo, args);
    const controller = new AbortController();
    const intervalId = setInterval(() => {
        ping(daemon);
    }, 5000).unref();

    controller.signal.addEventListener('abort', () => {
        clearInterval(intervalId);
    });

    return controller;
}
