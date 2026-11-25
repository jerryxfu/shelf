const UNITS = ["KB", "MB", "GB", "TB"];

/** A size as Finder writes it: 1 KB is 1000 bytes, one decimal below 100. */
export function formatSize(bytes: number): string {
    if (bytes < 1000) return bytes === 1 ? "1 byte" : `${bytes} bytes`;

    let value = bytes / 1000;
    let unit = 0;
    while (value >= 1000 && unit < UNITS.length - 1) {
        value /= 1000;
        unit++;
    }
    return `${value < 100 ? value.toFixed(1) : Math.round(value)} ${UNITS[unit]}`;
}
