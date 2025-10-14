'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from typing import TypeVar, Type
from pathlib import Path

from pydantic import BaseModel, ConfigDict

T = TypeVar('T', bound='StormbirdSetupBaseModel')

class StormbirdSetupBaseModel(BaseModel):
    '''
    Base class for the classes that define the setup of stormbird simulations. 
    '''
    model_config = ConfigDict(
        frozen = False,
        validate_assignment = True,
        extra = 'forbid',
        populate_by_name = True,
        use_enum_values=False,
        validate_default=True
    )

    @classmethod
    def from_json_string(cls: Type[T], json_string: str) -> T:
        return cls.model_validate_json(json_string)

    @classmethod
    def from_json_file(cls: Type[T], file_path: Path) -> T:
        return cls.model_validate_json(file_path.read_text())
    
    def to_json_string(self) -> str:
        return self.model_dump_json(exclude_none=True)

    def to_json_file(self, file_path: Path) -> None:
        file_path.write_text(self.to_json_string())

    def to_dict(self) -> dict:
        return self.model_dump(exclude_none=True)
