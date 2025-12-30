<script lang="ts">
import { onMount } from "svelte";
import { invoke } from "@tauri-apps/api/core";

// マイグレーション状態レポートの型定義
interface MigrationStatusReport {
	total_available: number;
	total_applied: number;
	pending_migrations: string[];
	last_migration_date: string | null;
	database_version: string;
}

// マイグレーション情報の型定義
interface MigrationInfo {
	name: string;
	version: string;
	description: string;
	checksum: string;
	is_applied: boolean;
	applied_at: string | null;
	execution_time_ms: number | null;
}

// 適用済みマイグレーションの型定義
interface AppliedMigration {
	id: number;
	name: string;
	version: string;
	description: string | null;
	checksum: string;
	applied_at: string;
	execution_time_ms: number | null;
	created_at: string;
}

// データベース統計の型定義
interface DatabaseStats {
	expenses_count: number;
	subscriptions_count: number;
	receipt_cache_count: number;
	categories_count: number;
	users_count: number;
	sessions_count: number;
	database_size_bytes: number;
	page_count: number;
	page_size: number;
	migrations_count: number | null;
}

// 詳細マイグレーション情報の型定義
interface DetailedMigrationInfo {
	status_report: MigrationStatusReport;
	available_migrations: MigrationInfo[];
	applied_migrations: AppliedMigration[];
	integrity_status: string;
	database_stats: DatabaseStats;
}

let statusReport: MigrationStatusReport | null = $state(null);
let detailedInfo: DetailedMigrationInfo | null = $state(null);
let loading = $state(false);
let error = $state("");

// マイグレーション状態を取得
async function fetchMigrationStatus() {
	loading = true;
	error = "";

	try {
		const result = await invoke<MigrationStatusReport>(
			"check_auto_migration_status",
		);
		statusReport = result;
	} catch (e) {
		error = `マイグレーション状態の取得に失敗しました: ${e}`;
		console.error("マイグレーション状態取得エラー:", e);
	} finally {
		loading = false;
	}
}

// 詳細マイグレーション情報を取得
async function fetchDetailedMigrationInfo() {
	loading = true;
	error = "";

	try {
		const result = await invoke<DetailedMigrationInfo>(
			"get_detailed_migration_info",
		);
		detailedInfo = result;
	} catch (e) {
		error = `詳細マイグレーション情報の取得に失敗しました: ${e}`;
		console.error("詳細マイグレーション情報取得エラー:", e);
	} finally {
		loading = false;
	}
}

// ページ読み込み時に状態を取得
onMount(() => {
	fetchMigrationStatus();
});

// ファイルサイズを人間が読みやすい形式に変換
function formatFileSize(bytes: number): string {
	if (bytes === 0) return "0 B";
	const k = 1024;
	const sizes = ["B", "KB", "MB", "GB"];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

// 実行時間を人間が読みやすい形式に変換
function formatExecutionTime(ms: number | null): string {
	if (ms === null) return "不明";
	if (ms < 1000) return `${ms}ms`;
	return `${(ms / 1000).toFixed(2)}s`;
}

// 日時を日本語形式に変換
function formatDateTime(dateStr: string | null): string {
	if (!dateStr) return "未適用";
	try {
		const date = new Date(dateStr);
		return date.toLocaleString("ja-JP", {
			year: "numeric",
			month: "2-digit",
			day: "2-digit",
			hour: "2-digit",
			minute: "2-digit",
			second: "2-digit",
			timeZone: "Asia/Tokyo",
		});
	} catch {
		return dateStr;
	}
}
</script>

<div class="container mx-auto p-6">
    <h1 class="text-3xl font-bold mb-6">マイグレーション状態確認</h1>
    
    {#if error}
        <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
            {error}
        </div>
    {/if}
    
    <div class="flex gap-4 mb-6">
        <button 
            onclick={fetchMigrationStatus}
            disabled={loading}
            class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded disabled:opacity-50"
        >
            {loading ? '読み込み中...' : '基本状態を取得'}
        </button>
        
        <button 
            onclick={fetchDetailedMigrationInfo}
            disabled={loading}
            class="bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded disabled:opacity-50"
        >
            {loading ? '読み込み中...' : '詳細情報を取得'}
        </button>
    </div>
    
    {#if statusReport}
        <div class="bg-white shadow-md rounded-lg p-6 mb-6">
            <h2 class="text-2xl font-semibold mb-4">基本マイグレーション状態</h2>
            
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                <div class="bg-blue-50 p-4 rounded">
                    <div class="text-2xl font-bold text-blue-600">{statusReport.total_available}</div>
                    <div class="text-sm text-gray-600">利用可能</div>
                </div>
                
                <div class="bg-green-50 p-4 rounded">
                    <div class="text-2xl font-bold text-green-600">{statusReport.total_applied}</div>
                    <div class="text-sm text-gray-600">適用済み</div>
                </div>
                
                <div class="bg-yellow-50 p-4 rounded">
                    <div class="text-2xl font-bold text-yellow-600">{statusReport.pending_migrations.length}</div>
                    <div class="text-sm text-gray-600">未適用</div>
                </div>
                
                <div class="bg-purple-50 p-4 rounded">
                    <div class="text-lg font-bold text-purple-600">{statusReport.database_version}</div>
                    <div class="text-sm text-gray-600">DBバージョン</div>
                </div>
            </div>
            
            <div class="mb-4">
                <strong>最終マイグレーション日時:</strong> 
                {formatDateTime(statusReport.last_migration_date)}
            </div>
            
            {#if statusReport.pending_migrations.length > 0}
                <div>
                    <strong>未適用マイグレーション:</strong>
                    <ul class="list-disc list-inside mt-2">
                        {#each statusReport.pending_migrations as migration}
                            <li class="text-yellow-700">{migration}</li>
                        {/each}
                    </ul>
                </div>
            {:else}
                <div class="text-green-600 font-semibold">
                    ✅ すべてのマイグレーションが適用済みです
                </div>
            {/if}
        </div>
    {/if}
    
    {#if detailedInfo}
        <div class="bg-white shadow-md rounded-lg p-6 mb-6">
            <h2 class="text-2xl font-semibold mb-4">詳細マイグレーション情報</h2>
            
            <!-- データベース統計 -->
            <div class="mb-6">
                <h3 class="text-xl font-semibold mb-3">データベース統計</h3>
                <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <div class="bg-gray-50 p-3 rounded">
                        <div class="text-lg font-bold">{detailedInfo.database_stats.expenses_count}</div>
                        <div class="text-sm text-gray-600">経費</div>
                    </div>
                    <div class="bg-gray-50 p-3 rounded">
                        <div class="text-lg font-bold">{detailedInfo.database_stats.subscriptions_count}</div>
                        <div class="text-sm text-gray-600">サブスクリプション</div>
                    </div>
                    <div class="bg-gray-50 p-3 rounded">
                        <div class="text-lg font-bold">{detailedInfo.database_stats.users_count}</div>
                        <div class="text-sm text-gray-600">ユーザー</div>
                    </div>
                    <div class="bg-gray-50 p-3 rounded">
                        <div class="text-lg font-bold">{detailedInfo.database_stats.migrations_count ?? 0}</div>
                        <div class="text-sm text-gray-600">マイグレーション</div>
                    </div>
                </div>
                <div class="mt-3">
                    <strong>データベースサイズ:</strong> {formatFileSize(detailedInfo.database_stats.database_size_bytes)}
                </div>
            </div>
            
            <!-- 整合性状態 -->
            <div class="mb-6">
                <h3 class="text-xl font-semibold mb-3">データベース整合性</h3>
                <div class="p-3 rounded {detailedInfo.integrity_status.includes('問題はありません') ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'}">
                    {detailedInfo.integrity_status}
                </div>
            </div>
            
            <!-- 利用可能なマイグレーション一覧 -->
            <div class="mb-6">
                <h3 class="text-xl font-semibold mb-3">利用可能なマイグレーション</h3>
                <div class="overflow-x-auto">
                    <table class="min-w-full bg-white border border-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-4 py-2 text-left">名前</th>
                                <th class="px-4 py-2 text-left">バージョン</th>
                                <th class="px-4 py-2 text-left">説明</th>
                                <th class="px-4 py-2 text-left">状態</th>
                                <th class="px-4 py-2 text-left">適用日時</th>
                                <th class="px-4 py-2 text-left">実行時間</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each detailedInfo.available_migrations as migration}
                                <tr class="border-t">
                                    <td class="px-4 py-2 font-mono text-sm">{migration.name}</td>
                                    <td class="px-4 py-2">{migration.version}</td>
                                    <td class="px-4 py-2">{migration.description}</td>
                                    <td class="px-4 py-2">
                                        {#if migration.is_applied}
                                            <span class="bg-green-100 text-green-800 px-2 py-1 rounded text-sm">適用済み</span>
                                        {:else}
                                            <span class="bg-yellow-100 text-yellow-800 px-2 py-1 rounded text-sm">未適用</span>
                                        {/if}
                                    </td>
                                    <td class="px-4 py-2 text-sm">{formatDateTime(migration.applied_at)}</td>
                                    <td class="px-4 py-2 text-sm">{formatExecutionTime(migration.execution_time_ms)}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>
            
            <!-- 適用済みマイグレーション詳細 -->
            {#if detailedInfo.applied_migrations.length > 0}
                <div>
                    <h3 class="text-xl font-semibold mb-3">適用済みマイグレーション詳細</h3>
                    <div class="overflow-x-auto">
                        <table class="min-w-full bg-white border border-gray-200">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-4 py-2 text-left">ID</th>
                                    <th class="px-4 py-2 text-left">名前</th>
                                    <th class="px-4 py-2 text-left">バージョン</th>
                                    <th class="px-4 py-2 text-left">適用日時</th>
                                    <th class="px-4 py-2 text-left">実行時間</th>
                                    <th class="px-4 py-2 text-left">チェックサム</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each detailedInfo.applied_migrations as migration}
                                    <tr class="border-t">
                                        <td class="px-4 py-2">{migration.id}</td>
                                        <td class="px-4 py-2 font-mono text-sm">{migration.name}</td>
                                        <td class="px-4 py-2">{migration.version}</td>
                                        <td class="px-4 py-2 text-sm">{formatDateTime(migration.applied_at)}</td>
                                        <td class="px-4 py-2 text-sm">{formatExecutionTime(migration.execution_time_ms)}</td>
                                        <td class="px-4 py-2 font-mono text-xs">{migration.checksum.substring(0, 16)}...</td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .container {
        max-width: 1200px;
    }
</style>