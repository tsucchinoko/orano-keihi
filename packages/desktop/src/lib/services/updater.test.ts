import { describe, it, expect } from 'vitest';
import { UpdaterService } from './updater';

describe('UpdaterService', () => {
  describe('compareVersions', () => {
    it('同じバージョンの場合は0を返す', () => {
      expect(UpdaterService.compareVersions('1.0.0', '1.0.0')).toBe(0);
      expect(UpdaterService.compareVersions('2.5.3', '2.5.3')).toBe(0);
    });

    it('メジャーバージョンが大きい場合は1を返す', () => {
      expect(UpdaterService.compareVersions('2.0.0', '1.0.0')).toBe(1);
      expect(UpdaterService.compareVersions('10.0.0', '9.0.0')).toBe(1);
    });

    it('メジャーバージョンが小さい場合は-1を返す', () => {
      expect(UpdaterService.compareVersions('1.0.0', '2.0.0')).toBe(-1);
      expect(UpdaterService.compareVersions('9.0.0', '10.0.0')).toBe(-1);
    });

    it('マイナーバージョンが大きい場合は1を返す', () => {
      expect(UpdaterService.compareVersions('1.2.0', '1.1.0')).toBe(1);
      expect(UpdaterService.compareVersions('1.10.0', '1.9.0')).toBe(1);
    });

    it('マイナーバージョンが小さい場合は-1を返す', () => {
      expect(UpdaterService.compareVersions('1.1.0', '1.2.0')).toBe(-1);
      expect(UpdaterService.compareVersions('1.9.0', '1.10.0')).toBe(-1);
    });

    it('パッチバージョンが大きい場合は1を返す', () => {
      expect(UpdaterService.compareVersions('1.0.2', '1.0.1')).toBe(1);
      expect(UpdaterService.compareVersions('1.0.10', '1.0.9')).toBe(1);
    });

    it('パッチバージョンが小さい場合は-1を返す', () => {
      expect(UpdaterService.compareVersions('1.0.1', '1.0.2')).toBe(-1);
      expect(UpdaterService.compareVersions('1.0.9', '1.0.10')).toBe(-1);
    });

    it('vプレフィックスを正しく処理する', () => {
      expect(UpdaterService.compareVersions('v1.0.0', '1.0.0')).toBe(0);
      expect(UpdaterService.compareVersions('v2.0.0', 'v1.0.0')).toBe(1);
      expect(UpdaterService.compareVersions('v1.0.0', 'v2.0.0')).toBe(-1);
    });

    it('プレリリースバージョンを正しく処理する', () => {
      // プレリリースがない方が新しい
      expect(UpdaterService.compareVersions('1.0.0', '1.0.0-beta')).toBe(1);
      expect(UpdaterService.compareVersions('1.0.0-beta', '1.0.0')).toBe(-1);

      // プレリリース同士の比較
      expect(
        UpdaterService.compareVersions('1.0.0-beta.2', '1.0.0-beta.1')
      ).toBe(1);
      expect(UpdaterService.compareVersions('1.0.0-alpha', '1.0.0-beta')).toBe(
        -1
      );
    });

    it('異なる長さのバージョン番号を正しく処理する', () => {
      expect(UpdaterService.compareVersions('1.0', '1.0.0')).toBe(0);
      expect(UpdaterService.compareVersions('1.0.0', '1.0')).toBe(0);
      expect(UpdaterService.compareVersions('1.0.1', '1.0')).toBe(1);
      expect(UpdaterService.compareVersions('1.0', '1.0.1')).toBe(-1);
    });
  });

  describe('formatFileSize', () => {
    it('バイト単位を正しくフォーマットする', () => {
      expect(UpdaterService.formatFileSize(500)).toBe('500.0 B');
      expect(UpdaterService.formatFileSize(1023)).toBe('1023.0 B');
    });

    it('キロバイト単位を正しくフォーマットする', () => {
      expect(UpdaterService.formatFileSize(1024)).toBe('1.0 KB');
      expect(UpdaterService.formatFileSize(1536)).toBe('1.5 KB');
      expect(UpdaterService.formatFileSize(1024 * 1023)).toBe('1023.0 KB');
    });

    it('メガバイト単位を正しくフォーマットする', () => {
      expect(UpdaterService.formatFileSize(1024 * 1024)).toBe('1.0 MB');
      expect(UpdaterService.formatFileSize(1024 * 1024 * 2.5)).toBe('2.5 MB');
    });

    it('ギガバイト単位を正しくフォーマットする', () => {
      expect(UpdaterService.formatFileSize(1024 * 1024 * 1024)).toBe('1.0 GB');
      expect(UpdaterService.formatFileSize(1024 * 1024 * 1024 * 1.5)).toBe(
        '1.5 GB'
      );
    });
  });

  describe('formatTimestamp', () => {
    it('Unix timestampを日本語形式でフォーマットする', () => {
      // 2024年1月1日 12:00:00 JST
      const timestamp = 1704081600;
      const formatted = UpdaterService.formatTimestamp(timestamp);

      // 日本語形式の日時文字列が含まれることを確認
      expect(formatted).toMatch(/2024/);
      expect(formatted).toMatch(/01/);
      // 時刻はタイムゾーンによって異なる可能性があるため、時刻の存在のみ確認
      expect(formatted).toMatch(/\d{2}:\d{2}/);
    });
  });
});
