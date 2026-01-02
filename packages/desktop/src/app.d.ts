// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	// 環境変数の型定義
	interface ImportMetaEnv {
		readonly R2_ACCOUNT_ID: string;
		readonly R2_ACCESS_KEY: string;
		readonly R2_SECRET_KEY: string;
		readonly R2_BUCKET_NAME: string;
		readonly R2_REGION: string;
	}

	interface ImportMeta {
		readonly env: ImportMetaEnv;
	}
}

export {};
