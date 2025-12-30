// マイグレーション状態確認コマンドのテストスクリプト
// このスクリプトは開発環境でTauriアプリケーションが起動している状態で実行します

const { invoke } = require("@tauri-apps/api/core");

async function testMigrationCommands() {
	console.log("マイグレーション状態確認コマンドのテストを開始します...\n");

	try {
		// 1. 基本的なマイグレーション状態を確認
		console.log("1. 基本マイグレーション状態を確認中...");
		const statusReport = await invoke("check_auto_migration_status");
		console.log(
			"基本マイグレーション状態:",
			JSON.stringify(statusReport, null, 2),
		);
		console.log("");

		// 2. 詳細なマイグレーション情報を取得
		console.log("2. 詳細マイグレーション情報を取得中...");
		const detailedInfo = await invoke("get_detailed_migration_info");
		console.log("詳細マイグレーション情報:");
		console.log(
			"- 利用可能なマイグレーション数:",
			detailedInfo.status_report.total_available,
		);
		console.log(
			"- 適用済みマイグレーション数:",
			detailedInfo.status_report.total_applied,
		);
		console.log(
			"- 未適用マイグレーション数:",
			detailedInfo.status_report.pending_migrations.length,
		);
		console.log(
			"- データベースバージョン:",
			detailedInfo.status_report.database_version,
		);
		console.log("- 整合性状態:", detailedInfo.integrity_status);
		console.log(
			"- データベースサイズ:",
			detailedInfo.database_stats.database_size_bytes,
			"bytes",
		);
		console.log("");

		// 3. 利用可能なマイグレーション一覧を表示
		console.log("3. 利用可能なマイグレーション一覧:");
		detailedInfo.available_migrations.forEach((migration, index) => {
			console.log(`  ${index + 1}. ${migration.name} (v${migration.version})`);
			console.log(`     説明: ${migration.description}`);
			console.log(`     状態: ${migration.is_applied ? "適用済み" : "未適用"}`);
			if (migration.applied_at) {
				console.log(`     適用日時: ${migration.applied_at}`);
			}
			if (migration.execution_time_ms) {
				console.log(`     実行時間: ${migration.execution_time_ms}ms`);
			}
			console.log("");
		});

		// 4. 適用済みマイグレーション詳細を表示
		if (detailedInfo.applied_migrations.length > 0) {
			console.log("4. 適用済みマイグレーション詳細:");
			detailedInfo.applied_migrations.forEach((migration, index) => {
				console.log(
					`  ${index + 1}. ID: ${migration.id}, 名前: ${migration.name}`,
				);
				console.log(`     バージョン: ${migration.version}`);
				console.log(`     適用日時: ${migration.applied_at}`);
				console.log(
					`     実行時間: ${migration.execution_time_ms || "不明"}ms`,
				);
				console.log(
					`     チェックサム: ${migration.checksum.substring(0, 16)}...`,
				);
				console.log("");
			});
		} else {
			console.log("4. 適用済みマイグレーションはありません");
		}

		console.log("✅ すべてのテストが正常に完了しました！");
	} catch (error) {
		console.error("❌ テスト中にエラーが発生しました:", error);
	}
}

// Node.js環境でのテスト実行
if (typeof window === "undefined") {
	console.log(
		"このスクリプトはTauriアプリケーション内で実行する必要があります。",
	);
	console.log("ブラウザの開発者ツールのコンソールで実行してください。");
} else {
	testMigrationCommands();
}
