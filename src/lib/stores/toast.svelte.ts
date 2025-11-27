/**
 * トースト通知の状態管理ストア
 */

export interface ToastMessage {
	id: number;
	message: string;
	type: "success" | "error" | "info";
}

class ToastStore {
	toasts = $state<ToastMessage[]>([]);
	private nextId = 0;

	/**
	 * トースト通知を表示する
	 */
	show(message: string, type: "success" | "error" | "info" = "info"): void {
		const id = this.nextId++;
		this.toasts = [...this.toasts, { id, message, type }];
	}

	/**
	 * 成功メッセージを表示する
	 */
	success(message: string): void {
		this.show(message, "success");
	}

	/**
	 * エラーメッセージを表示する
	 */
	error(message: string): void {
		this.show(message, "error");
	}

	/**
	 * 情報メッセージを表示する
	 */
	info(message: string): void {
		this.show(message, "info");
	}

	/**
	 * トースト通知を削除する
	 */
	remove(id: number): void {
		this.toasts = this.toasts.filter((toast) => toast.id !== id);
	}

	/**
	 * すべてのトースト通知をクリアする
	 */
	clear(): void {
		this.toasts = [];
	}
}

// シングルトンインスタンスをエクスポート
export const toastStore = new ToastStore();
