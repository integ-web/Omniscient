import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// DOM Elements
const form = document.getElementById("research-form") as HTMLFormElement;
const queryInput = document.getElementById("query-input") as HTMLInputElement;
const depthSelect = document.getElementById("depth-select") as HTMLSelectElement;
const searchContainer = document.querySelector(".search-container") as HTMLElement;
const progressContainer = document.getElementById("progress-container") as HTMLElement;
const logList = document.getElementById("log-list") as HTMLElement;
const reportContainer = document.getElementById("report-container") as HTMLElement;
const reportContent = document.getElementById("report-content") as HTMLElement;
const closeReportBtn = document.getElementById("close-report-btn") as HTMLButtonElement;

// Form Submission
form.addEventListener("submit", async (e) => {
  e.preventDefault();
  
  const query = queryInput.value.trim();
  const depth = depthSelect.value;
  
  if (!query) return;

  // UI Transitions
  searchContainer.classList.add("active");
  progressContainer.classList.remove("hidden");
  reportContainer.classList.add("hidden");
  logList.innerHTML = "";
  addLog("Agent initialized. Starting research pipeline...", "start");

  try {
    // Call Rust Backend Command
    const reportHTML = await invoke<string>("start_research", { query, depth });
    
    // Show Report
    progressContainer.classList.add("hidden");
    reportContainer.classList.remove("hidden");
    reportContent.innerHTML = reportHTML;
    
  } catch (err) {
    addLog(`Error: ${err}`, "error");
    progressContainer.querySelector('.spinner')?.classList.add('hidden');
  }
});

// Close Report
closeReportBtn.addEventListener("click", () => {
  reportContainer.classList.add("hidden");
  searchContainer.classList.remove("active");
  queryInput.value = "";
  queryInput.focus();
});

// Listen to Rust Events
listen<string>("research-progress", (event) => {
  addLog(event.payload, "info");
});

function addLog(msg: string, type: "info" | "error" | "start") {
  const el = document.createElement("div");
  el.className = `log-entry ${type}`;
  
  const time = new Date().toLocaleTimeString([], { hour12: false, hour: '2-digit', minute:'2-digit', second:'2-digit' });
  
  el.innerHTML = `<span class="log-time">[${time}]</span> <span class="log-msg">${escapeHtml(msg)}</span>`;
  logList.appendChild(el);
  
  // Auto-scroll to bottom
  logList.scrollTop = logList.scrollHeight;
}

function escapeHtml(unsafe: string) {
  return unsafe
       .replace(/&/g, "&amp;")
       .replace(/</g, "&lt;")
       .replace(/>/g, "&gt;")
       .replace(/"/g, "&quot;")
       .replace(/'/g, "&#039;");
}
