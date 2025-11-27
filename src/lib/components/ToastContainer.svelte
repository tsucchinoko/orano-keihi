<script lang="ts">
/**
 * トースト通知コンテナ
 * すべてのトースト通知を表示する
 */
import { toastStore } from "$lib/stores/toast.svelte";
import Toast from "./Toast.svelte";

const toasts = $derived(toastStore.toasts);
</script>

<div class="fixed top-4 right-4 z-50 space-y-3 pointer-events-none">
	{#each toasts as toast (toast.id)}
		<div class="pointer-events-auto">
			<Toast
				message={toast.message}
				type={toast.type}
				onClose={() => toastStore.remove(toast.id)}
			/>
		</div>
	{/each}
</div>
