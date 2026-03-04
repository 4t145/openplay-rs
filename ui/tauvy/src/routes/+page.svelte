<script lang="ts">
	import ScenePreview from "$lib/components/scene-preview.svelte";
  import { invoke } from "@tauri-apps/api/core";
  let name = $state("");
  let greetMsg = $state("");

  async function greet(event: Event) {
    event.preventDefault();
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg = await invoke("greet", { name });
  }
</script>
<ScenePreview>
  
</ScenePreview>
