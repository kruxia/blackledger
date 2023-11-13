from pydantic import SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict


class DatabaseSettings(BaseSettings):
    url: SecretStr
    dialect: str

    model_config = SettingsConfigDict(env_prefix="DATABASE_")
