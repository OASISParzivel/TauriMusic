import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";
import { closeCtx } from "./stores/context";

createApp(App).mount("#app");

// 屏蔽 WebView2 的浏览器右键菜单(另存为/刷新/更多工具等);
// 文本输入框保留系统的剪切/复制/粘贴菜单;空白区域右键同时收起自定义菜单
document.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement | null;
  if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
  e.preventDefault();
  closeCtx();
});
