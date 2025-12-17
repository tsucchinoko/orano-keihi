<script lang="ts">
/**
 * トースト通知コンポーネント
 * 成功・エラーメッセージを表示する
 */

interface Props {
	message: string;
	type?: "success" | "error" | "info" | "warning";
	duration?: number;
	onClose: () => void;
}

let { message, type = "info", duration = 3000, onClose }: Props = $props();

// 自動的に閉じる
$effect(() => {
	const timer = setTimeout(() => {
		onClose();
	}, duration);

	return () => clearTimeout(timer);
});

// タイプ別のスタイル
const typeStyles = {
	success: "bg-gradient-to-r from-green-500 to-emerald-500",
	error: "bg-gradient-to-r from-red-500 to-pink-500",
	info: "bg-gradient-to-r from-blue-500 to-cyan-500",
	warning: "bg-gradient-to-r from-yellow-500 to-orange-500",
};

// タイプ別のアイコン
const typeIcons = {
	success: "✓",
	error: "✕",
	info: "ℹ",
	warning: "⚠",
};
</script>

<div
	class="animate-slide-in"
	role="alert"
>
	<div class="rounded-lg p-4 {typeStyles[type]} text-white shadow-lg max-w-md">
		<div class="flex items-center gap-3">
			<div class="text-2xl">{typeIcons[type]}</div>
			<p class="flex-1">{message}</p>
			<button
				onclick={onClose}
				class="text-white hover:text-gray-200 transition-colors text-xl leading-none"
				aria-label="閉じる"
			>
				✕
			</button>
		</div>
	</div>
</div>

<style>
	@keyframes slide-in {
		from {
			transform: translateX(100%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.animate-slide-in {
		animation: slide-in 0.3s ease-out;
	}
</style>
