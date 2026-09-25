// Mounts an IndexedDB-backed /persist folder before main() runs so saves,
// settings, the booklet and best depth survive a reload, then flushes it to
// IndexedDB every few seconds and whenever the page is hidden.
Module.preRun = Module.preRun || [];
Module.preRun.push(function () {
  FS.mkdir("/persist");
  FS.mount(IDBFS, {}, "/persist");
  addRunDependency("persist");
  FS.syncfs(true, function (err) {
    if (err) console.warn("dreamscape: could not load saves", err);
    removeRunDependency("persist");
  });
  var syncing = false;
  function flush() {
    if (syncing) return;
    syncing = true;
    FS.syncfs(false, function () { syncing = false; });
  }
  setInterval(flush, 5000);
  document.addEventListener("visibilitychange", function () {
    if (document.visibilityState === "hidden") flush();
  });
  window.addEventListener("pagehide", flush);
});
