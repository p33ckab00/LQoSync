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
  link.addEventListener("click", () => {
    const target = document.getElementById(link.hash.slice(1));
    if (target instanceof HTMLDetailsElement) target.open = true;
  });
});
