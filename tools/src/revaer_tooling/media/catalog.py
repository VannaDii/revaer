"""Join validated manifest records to their locked source identities."""

from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from .diagnostics import verify_identity
from .model import Fixture, Source, document, fixtures, owned_path, sources
from .settings import FixtureSettings


@dataclass(frozen=True)
class Catalog:
    fixtures: tuple[Fixture, ...]
    sources: tuple[Source, ...]

    def source(self, name: str) -> Source:
        matches = tuple(source for source in self.sources if source.id == name)
        if len(matches) != 1:
            raise ToolingError(f"Fixture {name} has no unique locked source")
        return matches[0]


def load_catalog(root: Path, settings: FixtureSettings) -> Catalog:
    lock, entries = document(owned_path(root, settings.lock), "sources")
    _, manifest = document(owned_path(root, settings.manifest), "fixtures")
    catalog = Catalog(fixtures(manifest), sources(entries))
    downloaded = tuple(item for item in catalog.fixtures if item.download)
    if {item.id for item in downloaded} != {item.id for item in catalog.sources}:
        raise ToolingError("Fixture manifest and source lock must identify the same downloads")
    for fixture in downloaded:
        source = catalog.source(fixture.id)
        if source.path != fixture.path:
            raise ToolingError(f"Fixture {fixture.id} path differs from its source lock")
        if fixture.exact_diagnostic:
            entry = next(item for item in entries if item["id"] == fixture.id)
            verify_identity(fixture, source, lock, entry)
    return catalog
