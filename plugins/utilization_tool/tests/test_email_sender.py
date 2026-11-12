from unittest.mock import MagicMock, patch

import pandas as pd
import pytest
from auditor_utilization_plugin.email_sender import send_email


@pytest.mark.asyncio
@pytest.mark.parametrize("report_period", ["", "October 2026", "December 2025"])
@patch("auditor_utilization_plugin.email_sender.smtplib.SMTP")
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_success(mock_getenv, mock_smtp, report_period):
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

    await send_email("smtp.test.com", 587, df, logger, report_period=report_period)

    mock_server.login.assert_called_once_with("sender@test.com", "password")
    mock_server.send_message.assert_called_once()
    message = mock_server.send_message.call_args.args[0]
    suffix = f" - {report_period}" if report_period else ""
    assert message["Subject"] == f"Monthly Utilisation Report{suffix}"
    assert f"<h2>Monthly Utilisation Summary{suffix}</h2>" in (
        message.get_payload()[0].get_payload()
    )
    logger.info.assert_called_once()


@pytest.mark.asyncio
@patch("auditor_utilization_plugin.email_sender.smtplib.SMTP")
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_without_period_preserves_subject(mock_getenv, mock_smtp):
    mock_getenv.side_effect = {
        "SENDER_EMAIL": "sender@test.com",
        "PASSWORD": "password",
        "RECEIVER_EMAIL": "receiver@test.com",
    }.get

    await send_email("smtp.test.com", 587, pd.DataFrame(), MagicMock())

    server = mock_smtp.return_value.__enter__.return_value
    message = server.send_message.call_args.args[0]
    assert message["Subject"] == "Monthly Utilisation Report"


@pytest.mark.asyncio
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_missing_env(mock_getenv):
    mock_getenv.return_value = None

    df = pd.DataFrame({"user": ["test"]})
    logger = MagicMock()

    with pytest.raises(ValueError):
        await send_email("smtp.test.com", 587, df, logger)
