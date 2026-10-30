"use strict";

const sidebar = document.getElementById("sidebar");
const menu = document.getElementById("menuBtn");
const density = document.getElementById("densityBtn");

function closeMenu() {
  if (sidebar.contains(document.activeElement)) menu.focus();
  sidebar.classList.remove("open");
  menu.setAttribute("aria-expanded", "false");
}

menu.addEventListener("click", () => {
  const open = sidebar.classList.toggle("open");
  menu.setAttribute("aria-expanded", String(open));
  if (open) sidebar.querySelector('a[aria-current="page"]')?.focus();
});
document.addEventListener("click", (event) => {
  if (!sidebar.contains(event.target) && !menu.contains(event.target)) closeMenu();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    const open = sidebar.classList.contains("open");
    closeMenu();
    if (open) menu.focus();
  }
});

function setDensity(roomy) {
  document.body.classList.toggle("is-roomy", roomy);
  density.textContent = roomy ? "▤ Comfortable" : "▤ Compact";
  density.setAttribute("aria-pressed", String(roomy));
}
try { setDensity(localStorage.getItem("lqosync-density") === "comfortable"); } catch {}
density.addEventListener("click", () => {
  const roomy = !document.body.classList.contains("is-roomy");
  setDensity(roomy);
  try { localStorage.setItem("lqosync-density", roomy ? "comfortable" : "compact"); } catch {}
});

document.querySelectorAll("[data-filter]").forEach((input) => {
  input.addEventListener("input", () => {
    const query = input.value.trim().toLocaleLowerCase();
    document.querySelectorAll(input.dataset.filter).forEach((row) => {
      row.hidden = !row.textContent.toLocaleLowerCase().includes(query);
    });
  });
});

document.querySelectorAll('a[href^="#"]').forEach((link) => {
  link.addEventListener("click", (event) => {
    const target = document.getElementById(link.hash.slice(1));
    if (target instanceof HTMLDetailsElement) target.open = true;
    const trigger=target?.querySelector(".configcog");
    if (trigger) {event.preventDefault();trigger.click();}
  });
});

// Native dialogs keep configuration outside the page's stacking contexts.
const configurationPaths = /^\/(?:nodes\/save|nodes\/[^/]+\/delete|routers\/save|routers\/[^/]+\/delete|settings\/save|sources\/save|sources\/selection(?:\/[^/]+\/delete)?|wans\/save|assignments\/(?:pool|pin)|speed-rules\/(?:save|dhcp)|core-lists\/save|dynamic\/[^/]+)$/;
let activeEditor = null;
let editorScroll = "";

function configurationEditor(form, index) {
  const path = new URL(form.getAttribute("action"), location.href);
  if (form.getAttribute("method").toLowerCase() !== "post" || path.origin !== location.origin || !configurationPaths.test(path.pathname)) return;
  const section = form.closest("section, .inspect");
  const action = form.querySelector('button[type="submit"], button:not([type])')?.textContent.trim() || "Configure";
  const label = form.dataset.editorTitle || `${section?.querySelector("h2, h3")?.textContent.trim() || "configuration"} · ${action}`;
  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "iconbtn configcog";
  const visibleAction = action.match(/^(Promote|Move|Remove|Delete|Clear|Add|Pin)\b/);
  trigger.textContent = visibleAction ? `⚙ ${visibleAction[1]}` : "⚙";
  trigger.title = `Configure ${label}`;
  trigger.setAttribute("aria-label", `Configure ${label}`);
  trigger.setAttribute("aria-haspopup", "dialog");
  const summary = document.createElement("span");
  summary.className = "configsummary";
  const values = [...form.elements].filter(control =>
    control instanceof HTMLInputElement || control instanceof HTMLSelectElement
  ).filter(control => control.type !== "hidden" && control.type !== "password" &&
    !/password|credential|secret|token/i.test(control.name) &&
    (control.type !== "checkbox" || control.checked)
  ).map(control => control instanceof HTMLSelectElement ? control.selectedOptions[0]?.textContent.trim() : control.value).filter(Boolean);
  summary.textContent = values.slice(0, 3).join(" · ");
  const host = document.createElement("div");
  host.className = "configcontrol";
  host.append(summary, trigger);
  form.before(host);
  const dialog = document.createElement("dialog");
  dialog.className = "configdialog";
  dialog.setAttribute("aria-labelledby", `config-title-${index}`);
  const header = document.createElement("header");
  header.className = "confighead";
  const title = document.createElement("h2");
  title.id = `config-title-${index}`;
  title.textContent = label;
  title.tabIndex = -1;
  const close = document.createElement("button");
  close.type = "button"; close.className = "iconbtn"; close.textContent = "×";
  close.setAttribute("aria-label", `Close ${label}`);
  header.append(title, close);
  const body = document.createElement("div"); body.className = "configbody";
  const errors = document.createElement("div"); errors.className = "warning"; errors.hidden = true;
  errors.setAttribute("role", "alert"); errors.tabIndex = -1;
  const draft = document.createElement("p"); draft.className = "help";
  draft.textContent = "Closing keeps unsaved values in this dialog until you leave or reload the page.";
  body.append(errors, draft, form);
  const footer = document.createElement("footer"); footer.className = "configfooter";
  const cancel = document.createElement("button"); cancel.type = "button"; cancel.textContent = "Cancel";
  footer.append(cancel);
  const formId = form.getAttribute("id") || `config-form-${index}`;
  form.setAttribute("id", formId);
  for (const button of [...form.querySelectorAll("button")]) {
    if (button.type === "submit") { button.setAttribute("form", formId); footer.append(button); }
  }
  dialog.append(header, body, footer);
  document.body.append(dialog);
  let busy = false;
  const fieldErrors = [];
  function clearFieldErrors() {
    for (const {control, note, description} of fieldErrors.splice(0)) {
      control.removeAttribute("aria-invalid");
      if (description === null) control.removeAttribute("aria-describedby");
      else control.setAttribute("aria-describedby", description);
      note.remove();
    }
  }
  function showFieldErrors(message) {
    for (const control of form.elements) {
      if (!(control instanceof HTMLInputElement || control instanceof HTMLSelectElement) || control.type === "hidden") continue;
      const words = control.name.split("_").filter(word => word.length > 3 && word !== "original");
      if (!words.some(word => new RegExp(`\\b${word}\\b`, "i").test(message))) continue;
      const note = document.createElement("span");
      note.className = "field-error"; note.id = `config-field-error-${index}-${fieldErrors.length}`;
      note.textContent = message;
      const description = control.getAttribute("aria-describedby");
      control.setAttribute("aria-invalid", "true");
      control.setAttribute("aria-describedby", [description, note.id].filter(Boolean).join(" "));
      control.after(note); fieldErrors.push({control, note, description});
    }
  }
  function dismiss() { if (!busy) dialog.close(); }
  close.addEventListener("click", dismiss); cancel.addEventListener("click", dismiss);
  dialog.addEventListener("cancel", event => { if (busy) event.preventDefault(); });
  dialog.addEventListener("keydown", event => {
    if (event.key !== "Tab") return;
    const controls=[...dialog.querySelectorAll('button, input:not([type="hidden"]), select, textarea, a[href], [tabindex="0"]')].filter(control=>!control.disabled && control.getClientRects().length);
    const first=controls[0], last=controls[controls.length-1];
    if (!first) {event.preventDefault();title.focus();return;}
    if (event.shiftKey && (document.activeElement===first || !controls.includes(document.activeElement))) {event.preventDefault();last.focus();}
    else if (!event.shiftKey && (document.activeElement===last || !controls.includes(document.activeElement))) {event.preventDefault();first.focus();}
  });
  dialog.addEventListener("close", () => {
    if (activeEditor === dialog) { activeEditor = null; document.body.style.overflow = editorScroll; }
    const destination = trigger.isConnected ? trigger : document.querySelector(".configcog, main h1");
    destination?.focus();
  });
  trigger.addEventListener("click", () => {
    if (activeEditor) activeEditor.close();
    closeMenu(); editorScroll = document.body.style.overflow;
    document.body.style.overflow = "hidden"; activeEditor = dialog;
    dialog.showModal(); title.focus();
  });
  form.addEventListener("submit", async event => {
    event.preventDefault(); if (busy || !form.reportValidity()) return;
    const data = new URLSearchParams(new FormData(form, event.submitter));
    clearFieldErrors();
    busy = true; errors.hidden = true; dialog.setAttribute("aria-busy", "true");
    const buttons = [...footer.querySelectorAll("button"), close];
    buttons.forEach(button => button.disabled = true);
    try {
      const response = await fetch(path.href, {method: "POST", body: data, credentials: "same-origin"});
      if (response.ok && response.redirected && new URL(response.url).origin === location.origin) {
        location.assign(response.url); return;
      }
      const text = await response.text();
      const doc = new DOMParser().parseFromString(text, "text/html");
      const section = [...doc.querySelectorAll("section")].find(section => section.querySelector("h2")?.textContent === "Settings were not changed");
      errors.textContent = (section?.querySelector("p")?.textContent || (response.status === 403 ? "Invalid form token. Reload the page before saving." : doc.body.textContent.trim()) || "The save did not complete. Check the values and retry.").slice(0, 4096);
      showFieldErrors(errors.textContent);
      errors.hidden = false; errors.focus();
    } catch {
      errors.textContent = "The save response is unavailable. Reload to check the saved state before retrying.";
      errors.hidden = false; errors.focus();
    } finally {
      busy = false; dialog.removeAttribute("aria-busy"); buttons.forEach(button => button.disabled = false);
    }
  });

}
if (typeof HTMLDialogElement !== "undefined" && HTMLDialogElement.prototype.showModal) {
  document.querySelectorAll('form[method="post"]').forEach(configurationEditor);
  for (const details of [...document.querySelectorAll("details")].reverse()) {
    if (!details.querySelector(".configcog") || details.querySelector("form")) continue;
    const group = document.createElement("div"); group.className = "configgroup"; group.id=details.id;
    const summary = details.querySelector(":scope > summary");
    if (summary) { const title = document.createElement("h3"); title.textContent = summary.textContent; group.append(title); summary.remove(); }
    group.append(...details.childNodes); details.replaceWith(group);
  }
}
