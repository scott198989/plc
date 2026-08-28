export type ReplayPackageExport = Readonly<{
  bytes: ArrayBuffer;
  packageHash: string;
}>;

export type ReplayVerificationReceipt = Readonly<{
  contentFingerprint: string;
  divergence: null;
  eventCount: number;
  expectedBoundaryCount: number;
  finalSnapshotHash: string;
  observedBoundaryCount: number;
  schemaVersion: 1;
  verified: true;
}>;
