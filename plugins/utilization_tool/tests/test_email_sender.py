from unittest.mock import MagicMock, patch

import pandas as pd
import pytest

from auditor_utilization_plugin.email_sender import send_email


@pytest.mark.asyncio
@patch("auditor_utilization_plugin.email_sender.smtplib.SMTP")
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_success(mock_getenv, mock_smtp):
    # Mock environment variables
    mock_getenv.side_effect = lambda key: {
        "SENDER_EMAIL": "sender@test.com",
        "PASSWORD": "password",
        "RECEIVER_EMAIL": "receiver@test.com",
    }.get(key)

    # Mock SMTP instance
    mock_server = MagicMock()
    mock_smtp.return_value.__enter__.return_value = mock_server

    df = pd.DataFrame({"user": ["test"], "khs23h": [1.23]})
    logger = MagicMock()

    await send_email("smtp.test.com", 587, df, logger)

    mock_server.login.assert_called_once_with("sender@test.com", "password")
    mock_server.send_message.assert_called_once()
    logger.info.assert_called_once()


@pytest.mark.asyncio
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_missing_env(mock_getenv):
    mock_getenv.return_value = None

    df = pd.DataFrame({"user": ["test"]})
    logger = MagicMock()

    with pytest.raises(ValueError):
        await send_email("smtp.test.com", 587, df, logger)
