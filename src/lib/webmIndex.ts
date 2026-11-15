// The seek index of a WebM file (decision 0037): where in the file each part of the video
// starts, so streaming can jump straight to a point in the middle instead of reading from the
// start.
//
// A WebM file is a tree of EBML elements: each is an ID, a size and its data, the IDs and
// sizes written as variable-length numbers (the count of leading zero bits in the first byte
// says how many bytes follow). The parts that matter here:
//
//   EBML header
//   Segment
//     SeekHead   where the other top-level elements are (Cues among them)
//     Info       TimecodeScale: how long one time unit is (1 ms by default)
//     Tracks
//     Cues       the index: CuePoint { CueTime, CueTrackPositions { CueClusterPosition } }
//     Cluster…   the video itself, a few seconds per cluster
//
// Cues are usually written at the end (ffmpeg's default; Comilito's conversions put them at the
// front). Positions in the SeekHead and Cues count from the start of the Segment's data.

const ID = {
  EBML: 0x1a45dfa3,
  Segment: 0x18538067,
  SeekHead: 0x114d9b74,
  Seek: 0x4dbb,
  SeekID: 0x53ab,
  SeekPosition: 0x53ac,
  Info: 0x1549a966,
  TimecodeScale: 0x2ad7b1,
  Cues: 0x1c53bb6b,
  CuePoint: 0xbb,
  CueTime: 0xb3,
  CueTrackPositions: 0xb7,
  CueClusterPosition: 0xf1,
  Cluster: 0x1f43b675,
};

export interface WebmIndex {
  /** Bytes [0, initEnd) are the header MediaSource needs before any cluster. */
  initEnd: number;
  /** Sorted by time: a cluster starting at `time` (seconds) is at byte `offset`. */
  cues: { time: number; offset: number }[];
}

/** Reads bytes [start, end] of the file. */
export type ReadRange = (start: number, end: number) => Promise<Uint8Array>;

interface Element {
  id: number;
  /** Where the element's data starts, and how long it is (-1: "unknown", runs to its parent's end). */
  dataStart: number;
  size: number;
}

/** One element header at `pos`, or null if it doesn't fit in `buf`. */
function readElement(buf: Uint8Array, pos: number): Element | null {
  if (pos >= buf.length) return null;
  // The ID keeps its length marker bits: 1 to 4 bytes.
  const idLength = leadingZeros(buf[pos]) + 1;
  if (idLength > 4 || pos + idLength > buf.length) return null;
  let id = 0;
  for (let i = 0; i < idLength; i++) id = id * 256 + buf[pos + i];
  pos += idLength;
  // The size drops its marker bit: 1 to 8 bytes. All value bits set means "unknown".
  if (pos >= buf.length) return null;
  const sizeLength = leadingZeros(buf[pos]) + 1;
  if (sizeLength > 8 || pos + sizeLength > buf.length) return null;
  let size = buf[pos] & (0xff >> sizeLength);
  let allOnes = size === 0xff >> sizeLength;
  for (let i = 1; i < sizeLength; i++) {
    size = size * 256 + buf[pos + i];
    allOnes &&= buf[pos + i] === 0xff;
  }
  return { id, dataStart: pos + sizeLength, size: allOnes ? -1 : size };
}

function leadingZeros(byte: number): number {
  for (let i = 0; i < 8; i++) if (byte & (0x80 >> i)) return i;
  return 8;
}

/** A big-endian unsigned number (up to 8 bytes; exact up to 2^53, plenty for files). */
function readUint(buf: Uint8Array, start: number, size: number): number {
  let value = 0;
  for (let i = 0; i < size; i++) value = value * 256 + buf[start + i];
  return value;
}

/** The children of the element whose data is buf[start, end). */
function* children(buf: Uint8Array, start: number, end: number): Generator<Element> {
  let pos = start;
  while (pos < end) {
    const el = readElement(buf, pos);
    if (!el || el.size < 0) return;
    yield el;
    pos = el.dataStart + el.size;
  }
}

/** The CuePoints of a Cues element's data, as (time, offset) with absolute byte offsets. */
function parseCues(buf: Uint8Array, start: number, end: number, segmentStart: number, scale: number) {
  const cues: { time: number; offset: number }[] = [];
  for (const point of children(buf, start, end)) {
    if (point.id !== ID.CuePoint) continue;
    let time: number | null = null;
    let position: number | null = null;
    for (const field of children(buf, point.dataStart, point.dataStart + point.size)) {
      if (field.id === ID.CueTime) time = readUint(buf, field.dataStart, field.size);
      if (field.id === ID.CueTrackPositions && position === null) {
        for (const p of children(buf, field.dataStart, field.dataStart + field.size)) {
          if (p.id === ID.CueClusterPosition) position = readUint(buf, p.dataStart, p.size);
        }
      }
    }
    if (time !== null && position !== null) cues.push({ time: (time * scale) / 1e9, offset: segmentStart + position });
  }
  return cues.sort((a, b) => a.time - b.time);
}

/** How much of the start of the file is read to find the header and the SeekHead. */
const HEAD = 256 * 1024;

/**
 * Finds the seek index of a WebM file of `size` bytes, or null if it has none (or isn't a
 * WebM this understands): then streaming just reads from the start, as before.
 */
export async function readWebmIndex(read: ReadRange, size: number): Promise<WebmIndex | null> {
  const head = await read(0, Math.min(size, HEAD) - 1);
  const ebml = readElement(head, 0);
  if (!ebml || ebml.id !== ID.EBML) return null;
  const segment = readElement(head, ebml.dataStart + ebml.size);
  if (!segment || segment.id !== ID.Segment) return null;
  const segmentStart = segment.dataStart;

  let scale = 1_000_000;
  let cuesAt: number | null = null;
  let cues: WebmIndex["cues"] | null = null;
  let initEnd: number | null = null;

  // The Segment's top-level children, up to the first Cluster (that's where the header ends).
  let pos = segmentStart;
  while (pos < head.length) {
    const el = readElement(head, pos);
    if (!el) break;
    if (el.id === ID.Cluster) {
      initEnd = pos;
      break;
    }
    if (el.size < 0) break;
    const end = el.dataStart + el.size;
    if (el.id === ID.SeekHead && end <= head.length) {
      for (const seek of children(head, el.dataStart, end)) {
        if (seek.id !== ID.Seek) continue;
        let target: number | null = null;
        let at: number | null = null;
        for (const f of children(head, seek.dataStart, seek.dataStart + seek.size)) {
          if (f.id === ID.SeekID) target = readUint(head, f.dataStart, f.size);
          if (f.id === ID.SeekPosition) at = readUint(head, f.dataStart, f.size);
        }
        if (target === ID.Cues && at !== null) cuesAt = segmentStart + at;
      }
    }
    if (el.id === ID.Info && end <= head.length) {
      for (const f of children(head, el.dataStart, end)) if (f.id === ID.TimecodeScale) scale = readUint(head, f.dataStart, f.size);
    }
    // Cues at the front, as Comilito's conversions write them.
    if (el.id === ID.Cues) cuesAt = pos;
    pos = end;
  }
  if (initEnd === null) return null;

  // Read the Cues where the SeekHead (or the walk above) found them.
  if (cuesAt !== null && cuesAt < size) {
    const header = await read(cuesAt, Math.min(size, cuesAt + 12) - 1);
    const el = readElement(header, 0);
    if (el && el.id === ID.Cues && el.size > 0) {
      const body = await read(cuesAt + el.dataStart, Math.min(size, cuesAt + el.dataStart + el.size) - 1);
      cues = parseCues(body, 0, body.length, segmentStart, scale);
    }
  }
  return cues && cues.length ? { initEnd, cues } : null;
}

/** The byte offset of the last cluster starting at or before `seconds`. */
export function offsetFor(index: WebmIndex, seconds: number): number {
  let best = index.cues[0];
  for (const cue of index.cues) {
    if (cue.time > seconds) break;
    best = cue;
  }
  return best.offset;
}
