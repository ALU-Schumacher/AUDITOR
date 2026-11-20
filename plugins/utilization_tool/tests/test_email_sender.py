import smtplib
import ssl
from unittest.mock import MagicMock, call, patch

import pandas as pd
import pytest
from auditor_utilization_plugin.email_sender import send_email


@pytest.mark.asyncio
@pytest.mark.parametrize("use_tls", [False, True])
@pytest.mark.parametrize("password", [None, "", "password"])
@patch("auditor_utilization_plugin.email_sender.smtplib.SMTP")
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_success(mock_getenv, mock_smtp, password, use_tls):
    # Mock environment variables
    mock_getenv.side_effect = lambda key: {
        "SENDER_EMAIL": "sender@test.com",
        "PASSWORD": password,
        "RECEIVER_EMAIL": "receiver@test.com",
    }.get(key)

    # Mock SMTP instance
    mock_server = MagicMock()
    mock_smtp.return_value.__enter__.return_value = mock_server

    df = pd.DataFrame({"user": ["test"], "khs23h": [1.23]})
    logger = MagicMock()

    await send_email("smtp.test.com", 587, df, logger, use_tls=use_tls)

    if use_tls:
        mock_server.starttls.assert_called_once()
        context = mock_server.starttls.call_args.kwargs["context"]
        assert isinstance(context, ssl.SSLContext)
        assert context.check_hostname is True
        assert context.verify_mode == ssl.CERT_REQUIRED
        mock_server.ehlo.assert_called_once_with()
        assert mock_server.method_calls[:2] == [
            call.starttls(context=context),
            call.ehlo(),
        ]
    else:
        mock_server.starttls.assert_not_called()

    if password:
        mock_server.login.assert_called_once_with("sender@test.com", password)
        assert mock_server.method_calls[-2] == call.login("sender@test.com", password)
    else:
        mock_server.login.assert_not_called()

    mock_server.send_message.assert_called_once()
    assert mock_server.method_calls[-1] == call.send_message(
        mock_server.send_message.call_args.args[0]
    )
    message = mock_server.send_message.call_args.args[0]
    assert message["From"] == "sender@test.com"
    assert message["To"] == "receiver@test.com"
    assert "test" in message.get_payload()[0].get_payload()
    logger.info.assert_called_once()


@pytest.mark.asyncio
@pytest.mark.parametrize("missing_name", ["SENDER_EMAIL", "RECEIVER_EMAIL"])
@pytest.mark.parametrize("missing_value", [None, ""])
@patch("auditor_utilization_plugin.email_sender.smtplib.SMTP")
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_missing_env(
    mock_getenv, mock_smtp, missing_name, missing_value
):
    values = {
        "SENDER_EMAIL": "sender@test.com",
        "RECEIVER_EMAIL": "receiver@test.com",
        "PASSWORD": "password",
        missing_name: missing_value,
    }
    mock_getenv.side_effect = values.get

    df = pd.DataFrame({"user": ["test"]})
    logger = MagicMock()

    with pytest.raises(ValueError, match="SENDER_EMAIL and RECEIVER_EMAIL"):
        await send_email("smtp.test.com", 587, df, logger)
    mock_smtp.assert_not_called()


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "error",
    [
        smtplib.SMTPNotSupportedError("STARTTLS not supported"),
        ssl.SSLCertVerificationError("Certificate verification failed"),
    ],
)
@patch("auditor_utilization_plugin.email_sender.smtplib.SMTP")
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_starttls_failure_aborts_delivery(mock_getenv, mock_smtp, error):
    mock_getenv.side_effect = {
        "SENDER_EMAIL": "sender@test.com",
        "RECEIVER_EMAIL": "receiver@test.com",
        "PASSWORD": "password",
    }.get
    server = mock_smtp.return_value.__enter__.return_value
    server.starttls.side_effect = error
    logger = MagicMock()

    with pytest.raises(type(error)):
        await send_email("smtp.test.com", 587, pd.DataFrame(), logger, use_tls=True)

    server.login.assert_not_called()
    server.send_message.assert_not_called()
    logger.info.assert_not_called()


@pytest.mark.asyncio
@patch("auditor_utilization_plugin.email_sender.smtplib.SMTP")
@patch("auditor_utilization_plugin.email_sender.os.getenv")
async def test_send_email_defaults_to_plain_smtp(mock_getenv, mock_smtp):
    mock_getenv.side_effect = {
        "SENDER_EMAIL": "sender@test.com",
        "RECEIVER_EMAIL": "receiver@test.com",
    }.get
    server = mock_smtp.return_value.__enter__.return_value

    await send_email("smtp.test.com", 25, pd.DataFrame(), MagicMock())

    server.starttls.assert_not_called()
    server.login.assert_not_called()
    server.send_message.assert_called_once()
