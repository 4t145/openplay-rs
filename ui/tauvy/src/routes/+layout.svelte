<script>import "../app.css";
import { page } from '$app/state';
import { locales, localizeHref } from '$lib/paraglide/runtime';
  import { SettingsIcon } from '@lucide/svelte';

import './layout.css';

const { children } = $props();
</script>

<main class="w-screen h-screen bg-[#0000AA] relative overflow-hidden">
	<div class="absolute top-0 right-0 w-full h-12 flex items-center justify-end gap-2 px-4 z-50">
		<button type="button" class="btn-icon preset-outlined-surface-500 text-white border-white hover:bg-white/20">
			<SettingsIcon/>
		</button>
	</div>

	<!-- Page Content -->
	<div class="w-full h-full pt-12 overflow-y-auto">
		{@render children()}
	</div>

	<div style="display:none">
		{#each locales as locale}
			<a href="{localizeHref(page.url.pathname, {locale})}">{locale}</a>
		{/each}
	</div>
</main>
