<script setup lang="ts">
import { ref } from "vue";
import Modal from "./Modal.vue";
import { acceptWelcome } from "../stores/ui";

const AUTHOR_MAIL = "O_Art3mis@163.com";

/** 两步引导:1 使用声明 → 2 感谢信 */
const step = ref<1 | 2>(1);
</script>

<template>
  <Modal @close="acceptWelcome">
    <Transition name="step" mode="out-in">
      <!-- 第一步:使用声明 -->
      <div v-if="step === 1" key="terms">
        <header class="head">
          <span class="logo">
            <svg viewBox="0 0 24 24">
              <path d="M9.3 17.6V6.9l9.4-2v10.5" fill="none" stroke="#fff" stroke-width="1.9" stroke-linejoin="round" />
              <circle cx="7" cy="17.7" r="2.5" fill="#fff" />
              <circle cx="16.4" cy="15.6" r="2.5" fill="#fff" />
            </svg>
          </span>
          <div>
            <h2>欢迎使用 TauriMusic</h2>
            <p class="sub">首次使用前,请阅读以下使用声明</p>
          </div>
        </header>

        <ul class="terms">
          <li>
            <b>本地播放</b>
            <span>本应用仅播放你导入的本地音频文件,不会修改或移动原始文件。</span>
          </li>
          <li>
            <b>隐私</b>
            <span>不收集、不上传任何用户数据;曲库与播放记录只保存在本机。</span>
          </li>
          <li>
            <b>在线歌词与封面</b>
            <span>歌词、封面通过公开网络接口按歌曲名查询,仅获取文本与图片元数据,不提供任何音频下载。</span>
          </li>
          <li>
            <b>音乐版权</b>
            <span>请确保导入的音乐为你合法获得。因使用本应用产生的版权问题由使用者自行承担。</span>
          </li>
        </ul>

        <button class="cta" @click="step = 2">同意并继续</button>
      </div>

      <!-- 第二步:感谢信 -->
      <div v-else key="letter" class="letter-wrap">
        <header class="head letter-head">
          <span class="logo">
            <svg viewBox="0 0 24 24">
              <path d="M9.3 17.6V6.9l9.4-2v10.5" fill="none" stroke="#fff" stroke-width="1.9" stroke-linejoin="round" />
              <circle cx="7" cy="17.7" r="2.5" fill="#fff" />
              <circle cx="16.4" cy="15.6" r="2.5" fill="#fff" />
            </svg>
          </span>
          <div>
            <h2>一封感谢信</h2>
            <p class="sub">来自开发者</p>
          </div>
        </header>

        <div class="letter">
          <p class="salut">致每一个打开 TauriMusic 的你:</p>
          <p>谢谢您愿意支持我的项目。</p>
          <p>
            它源于一个很简单的念头——音乐就该被干净、安静地播放,不被广告打扰,轻量的资源占用，理所当然的，TauriMusic属于你。
          </p>
          <p>这是我的第一个开源项目，因此希望您能对这个项目给予一些额外的宽容，我会继续努力完善它。</p>
          <p class="ps">如果您有任何好的想法和意见欢迎issue|Email。</p>
          <div class="sign">
            <span class="from">—— O_Art3mis</span>
            <a class="mail" :href="`mailto:${AUTHOR_MAIL}`">{{ AUTHOR_MAIL }}</a>
          </div>
        </div>

        <button class="cta" @click="acceptWelcome">开始使用</button>
      </div>
    </Transition>

    <div class="dots">
      <span class="dot" :class="{ on: step === 1 }"></span>
      <span class="dot" :class="{ on: step === 2 }"></span>
    </div>
  </Modal>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 14px;
}
.logo {
  width: 52px;
  height: 52px;
  border-radius: 13px;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
  flex: none;
}
.logo svg {
  width: 34px;
  height: 34px;
}
h2 {
  font-size: 18px;
  font-weight: 700;
  color: var(--text);
}
.sub {
  font-size: 12.5px;
  color: var(--text-2);
  margin-top: 3px;
}

.terms {
  display: grid;
  gap: 10px;
  margin-bottom: 16px;
}
.terms li {
  list-style: none;
  border: 1px solid var(--hairline);
  border-radius: 12px;
  padding: 11px 14px;
  display: grid;
  gap: 3px;
}
.terms b {
  font-size: 13px;
  color: var(--text);
}
.terms span {
  font-size: 12.5px;
  color: var(--text-2);
  line-height: 1.65;
}

/* ===== 感谢信 ===== */
.letter {
  border: 1px solid var(--hairline);
  border-radius: 12px;
  padding: 16px 18px;
  margin-bottom: 16px;
  font-size: 13px;
  color: var(--text);
  line-height: 1.9;
}
.letter p {
  margin-bottom: 8px;
}
.letter p:last-child {
  margin-bottom: 0;
}
.salut {
  font-weight: 600;
}
.ps {
  color: var(--text-2);
  font-size: 12.5px;
}
.sign {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
}
.from {
  font-weight: 600;
}
.mail {
  font-size: 12px;
  color: var(--accent);
  text-decoration: none;
}
.mail:hover {
  text-decoration: underline;
}

.cta {
  width: 100%;
  height: 42px;
  border-radius: 999px;
  background: var(--accent);
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  transition: filter 0.2s ease, transform 0.15s var(--ease-out-soft);
}
.cta:hover {
  filter: brightness(1.08);
}
.cta:active {
  transform: scale(0.98);
}

/* 步骤指示点 */
.dots {
  display: flex;
  justify-content: center;
  gap: 6px;
  margin-top: 12px;
}
.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--pill-bg);
  transition: background 0.25s ease, transform 0.25s var(--ease-spring);
}
.dot.on {
  background: var(--accent);
  transform: scale(1.15);
}

/* 步骤切换过渡 */
.step-enter-active {
  transition: opacity 0.28s var(--ease-out-soft), transform 0.34s var(--ease-out-soft);
}
.step-leave-active {
  transition: opacity 0.16s ease, transform 0.16s ease;
}
.step-enter-from {
  opacity: 0;
  transform: translateX(18px);
}
.step-leave-to {
  opacity: 0;
  transform: translateX(-14px);
}
</style>
