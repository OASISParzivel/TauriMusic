/* TauriMusic 宣传页交互:原生 JS,无依赖 */
(function () {
  "use strict";

  /* ---------- 深浅色切换(记忆选择) ---------- */
  var root = document.documentElement;
  try {
    var saved = localStorage.getItem("tm2-theme");
    if (saved === "light" || saved === "dark") root.setAttribute("data-theme", saved);
  } catch (e) { /* file:// 下隐私模式可能禁用,忽略 */ }
  var themeBtn = document.getElementById("themeBtn");
  if (themeBtn) {
    themeBtn.addEventListener("click", function () {
      var next = root.getAttribute("data-theme") === "dark" ? "light" : "dark";
      root.setAttribute("data-theme", next);
      try { localStorage.setItem("tm2-theme", next); } catch (e) {}
    });
  }

  /* ---------- 滚动入场 ---------- */
  var revealEls = document.querySelectorAll(".reveal");
  /* 兜底:已在视口内的直接显示(应对带锚点打开、IO 延迟等情况) */
  function sweep() {
    var vh = window.innerHeight || 800;
    revealEls.forEach(function (el) {
      var r = el.getBoundingClientRect();
      if (r.top < vh * 0.97 && r.bottom > 0) el.classList.add("in");
    });
  }
  if ("IntersectionObserver" in window) {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (en) {
        if (en.isIntersecting) {
          en.target.classList.add("in");
          io.unobserve(en.target);
        }
      });
    }, { threshold: 0.12 });
    revealEls.forEach(function (el) { io.observe(el); });
  }
  window.addEventListener("load", function () {
    sweep();
    setTimeout(sweep, 350);
  });
  sweep();

  /* ---------- 玻璃面板指针高光 ---------- */
  var raf = 0;
  document.addEventListener("pointermove", function (e) {
    if (raf) return;
    raf = requestAnimationFrame(function () {
      raf = 0;
      var el = e.target && e.target.closest ? e.target.closest(".glass, .glass-heavy") : null;
      document.querySelectorAll(".glass, .glass-heavy").forEach(function (g) {
        if (g === el) {
          var r = g.getBoundingClientRect();
          g.style.setProperty("--mx", ((e.clientX - r.left) / r.width) * 100 + "%");
          g.style.setProperty("--my", ((e.clientY - r.top) / r.height) * 100 + "%");
        }
      });
    });
  }, { passive: true });

  /* ---------- 统计数字滚动 ---------- */
  function animateCount(el) {
    var target = parseInt(el.getAttribute("data-count"), 10);
    if (isNaN(target)) return;
    var done = false;
    function finish() {
      if (done) return;
      done = true;
      el.textContent = target.toLocaleString("en-US");
    }
    var t0 = null;
    var dur = 1400;
    function step(ts) {
      if (done) return;
      if (!t0) t0 = ts;
      var p = Math.min(1, (ts - t0) / dur);
      var eased = 1 - Math.pow(1 - p, 3);
      el.textContent = Math.round(target * eased).toLocaleString("en-US");
      if (p < 1) requestAnimationFrame(step);
      else finish();
    }
    requestAnimationFrame(step);
    /* 兜底:rAF 被节流/虚拟时间截断时也能落到终值 */
    setTimeout(finish, dur + 400);
  }
  var counters = document.querySelectorAll("[data-count]");
  if ("IntersectionObserver" in window) {
    var cio = new IntersectionObserver(function (entries) {
      entries.forEach(function (en) {
        if (en.isIntersecting) {
          animateCount(en.target);
          cio.unobserve(en.target);
        }
      });
    }, { threshold: 0.4 });
    counters.forEach(function (el) { cio.observe(el); });
  } else {
    counters.forEach(animateCount);
  }

  /* ---------- 全屏播放页演示:进度 + 逐行歌词联动 ---------- */
  var DURATION = 269; /* 4:29,与演示曲目时长一致 */
  var playBtn = document.getElementById("npPlay");
  var fill = document.getElementById("npFill");
  var cur = document.getElementById("npCur");
  var track = document.getElementById("npTrack");
  var lyricsEl = document.getElementById("npLyrics");
  if (playBtn && fill && lyricsEl) {
    var lines = Array.prototype.slice.call(lyricsEl.querySelectorAll("li"));
    var pos = 0;
    var playing = false;
    var lastTs = 0;

    function fmt(s) {
      s = Math.max(0, Math.floor(s));
      return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0");
    }
    function render() {
      fill.style.width = (pos / DURATION) * 100 + "%";
      cur.textContent = fmt(pos);
      /* 歌词行按进度均分高亮(演示逻辑,真实实现读 LRC 时间戳) */
      var per = DURATION / lines.length;
      var active = Math.min(lines.length - 1, Math.floor(pos / per));
      lines.forEach(function (li, i) {
        li.classList.toggle("on", i === active);
        li.classList.toggle("done", i < active);
      });
      /* 让高亮行尽量停在可视区中部(overflow:hidden 也可编程滚动) */
      var li = lines[active];
      if (li) {
        lyricsEl.scrollTop = Math.max(0, li.offsetTop - lyricsEl.clientHeight / 2 + li.clientHeight);
      }
    }
    function tick(ts) {
      if (!playing) return;
      if (!lastTs) lastTs = ts;
      var dt = (ts - lastTs) / 1000;
      lastTs = ts;
      pos += dt * 8; /* 演示用 8 倍速,4 分半的歌约 34 秒过完 */
      if (pos >= DURATION) pos = 0; /* 循环演示 */
      render();
      requestAnimationFrame(tick);
    }
    playBtn.addEventListener("click", function () {
      playing = !playing;
      playBtn.classList.toggle("playing", playing);
      lastTs = 0;
      if (playing) requestAnimationFrame(tick);
    });
    /* 点进度条跳转 */
    if (track) {
      track.addEventListener("click", function (e) {
        var r = track.getBoundingClientRect();
        pos = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * DURATION;
        render();
      });
    }
    /* 点歌词行跳转(与真实产品行为一致) */
    lines.forEach(function (li, i) {
      li.addEventListener("click", function () {
        pos = (DURATION / lines.length) * i + 0.01;
        render();
      });
    });
    render();
  }
})();
