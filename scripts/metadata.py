"""Pretty print release metadata for diagnostics."""

from dataclasses import dataclass
import json
import sys


@dataclass
class DictInfo:
    path: str
    size: str
    size_bytes: float


def parse_size(size_str: str) -> float:
    size_str = size_str.strip()
    units = {
        "KB": 1024,
        "MB": 1024**2,
        "GB": 1024**3,
    }

    for unit, multiplier in units.items():
        if size_str.endswith(unit):
            value = float(size_str[: -len(unit)].strip())
            return value * multiplier
    # If no unit...
    assert False


def extract_dictionaries(data) -> list[DictInfo]:
    """Extract only actual dictionaries (source -> target pairs)."""
    results: list[DictInfo] = []

    def traverse_targets(source_path: str, targets_obj) -> None:
        if isinstance(targets_obj, dict):
            for target_name, target_value in targets_obj.items():
                if isinstance(target_value, str):
                    size_bytes = parse_size(target_value)
                    dict_info = DictInfo(
                        path=f"{source_path} -> {target_name}",
                        size=target_value,
                        size_bytes=size_bytes,
                    )
                    results.append(dict_info)

    if "dicts" in data:
        for dict_type, dict_data in data["dicts"].items():
            if "sources" in dict_data:
                for source_name, source_data in dict_data["sources"].items():
                    if "targets" in source_data:
                        source_path = f"{dict_type}/{source_name}"
                        traverse_targets(source_path, source_data["targets"])

    return results


def main() -> None:
    fmt = sys.argv[1] if len(sys.argv) > 1 else "yomitan"
    with open(f"docs/release_metadata_{fmt}.json", "r") as f:
        data = json.load(f)

    all_dicts = extract_dictionaries(data)
    all_dicts.sort(key=lambda x: x.size_bytes, reverse=True)

    upto = 10

    print("=" * 80)
    print(f"TOP {upto} LARGEST DICTIONARIES")
    print("=" * 80)
    print(f"{'Rank':<6} {'Size':<12} {'Dictionary'}")
    print("-" * 80)

    for i, dict_info in enumerate(all_dicts[:upto], 1):
        print(f"{i:<6} {dict_info.size:<12} {dict_info.path}")

    print("\n" + "=" * 80)
    print(f"Total dictionaries: {len(all_dicts)}")


if __name__ == "__main__":
    main()
