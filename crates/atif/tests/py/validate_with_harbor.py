import sys
from pathlib import Path

from harbor.utils.trajectory_validator import TrajectoryValidator


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: validate_with_harbor.py <trajectory.json> <valid|invalid>", file=sys.stderr)
        return 2

    trajectory_path = Path(sys.argv[1])
    expected = sys.argv[2]
    expect_valid = expected == "valid"

    validator = TrajectoryValidator()
    is_valid = validator.validate(trajectory_path, validate_images=False)

    if is_valid == expect_valid:
        return 0

    print(f"expected {expected}, got {'valid' if is_valid else 'invalid'}", file=sys.stderr)
    for error in validator.get_errors():
        print(error, file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
