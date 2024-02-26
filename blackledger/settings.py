from typing import Optional

from pydantic import Field, SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict


class DatabaseSettings(BaseSettings):
    url: SecretStr
    dialect: str

    model_config = SettingsConfigDict(env_prefix="DATABASE_")


class AuthSettings(BaseSettings):
    disabled: bool = False
    jwks_url: str
    iss: str
    alg: str = "RS256"
    aud: Optional[str] = None

    model_config = SettingsConfigDict(env_prefix="AUTH_")


class Settings(BaseSettings):
    debug: bool
    auth: AuthSettings = Field(default_factory=AuthSettings)
    db: DatabaseSettings = Field(default_factory=DatabaseSettings)
