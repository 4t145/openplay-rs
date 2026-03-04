<script lang="ts">
    import { onMount } from 'svelte';
    // 导入刚才通过 package.json 引入的本地模块
    import init, { start_bevy_app } from 'op-bevy';

    let canvasId = "bevy-canvas-preview";
    let isLoaded = false;

    onMount(async () => {
        try {
            // 1. 必须先通过 init() 加载 wasm 文件环境
            await init(); 
            isLoaded = true;

            // 2. 环境准备好后，调用你写的 Bevy 启动函数，并把 Canvas 的 id 传给它
            // 这里请确保你在 start_bevy_app 里做好了只初始化一次的逻辑
            start_bevy_app(canvasId); 
        } catch (e) {
            console.error("加载 Bevy Wasm 失败:", e);
        }
    });
</script>

<!-- 这个容器用于放置 bevy 渲染的画布 -->
<div class="absolute bottom-0 top-0 w-full h-full border-t border-white/20 bg-black/50 z-40">
    {#if !isLoaded}
        <div class="w-full h-full flex items-center justify-center text-white">
            正在加载 3D 渲染引擎...
        </div>
    {/if}
    <!-- Bevy 会在这里寻找 ID 为 bevy-canvas-preview 的画布并接管它 -->
    <canvas id={canvasId} class="w-full h-full outline-none block"></canvas>
</div>