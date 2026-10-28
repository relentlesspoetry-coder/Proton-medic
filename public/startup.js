(function () {
  function report(message) {
    var text = String(message).slice(0, 4000);
    console.error('[Proton Medic startup] ' + text);
    var bridge = window.__TAURI_INTERNALS__;
    if (bridge && bridge.invoke) bridge.invoke('startup_diagnostic', { message: text }).catch(function () {});
  }
  window.addEventListener('error', function (e) {
    var message = e.message || ('Resource failed: ' + (e.target && (e.target.src || e.target.href) || 'unknown'));
    report(message);
    var banner = document.getElementById('startup-status');
    if (banner) { banner.hidden = false; banner.textContent = 'Proton Medic could not start: ' + message; }
  }, true);
  window.addEventListener('unhandledrejection', function (e) { report('Unhandled promise: ' + String(e.reason)); });
  report('HTML startup script executed');
  document.addEventListener('DOMContentLoaded', function () { report('DOM loaded'); });
  setTimeout(function () {
    if (!document.querySelector('#root .pm-app')) report('App not mounted after 15 seconds');
    else report('Startup complete: application DOM is present (visible rendering still requires confirmation)');
  }, 15000);
})();
