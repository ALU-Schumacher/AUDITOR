import os
import smtplib
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from html import escape
from logging import Logger
from typing import Optional

import pandas as pd
from dotenv import load_dotenv

load_dotenv()


async def send_email(
    smtp_server: str,
    smtp_port: int,
    df_sum: pd.DataFrame,
    logger: Logger,
    metadata_filters: Optional[dict[str, str]] = None,
) -> None:
    try:
        sender_email = os.getenv("SENDER_EMAIL")
        app_password = os.getenv("PASSWORD")
        receiver_email = os.getenv("RECEIVER_EMAIL")

        if not sender_email or not app_password or not receiver_email:
            raise ValueError("Missing required email environment variables")

        html_table = df_sum.to_html(index=False, border=0, justify="center")

        filter_suffix = ""
        if metadata_filters:
            label = ", ".join(
                f"{key}={value}" for key, value in sorted(metadata_filters.items())
            )
            filter_suffix = " [" + " ".join(label.splitlines()) + "]"

        subject = "Monthly Utilisation Report"
        subject += filter_suffix
        body_html = f"""
        <html>
          <body>
            <h2>Monthly Utilisation Summary{escape(filter_suffix)}</h2>
            {html_table}
          </body>
        </html>
        """

        message = MIMEMultipart("alternative")
        message["Subject"] = subject
        message["From"] = sender_email
        message["To"] = receiver_email

        message.attach(MIMEText(body_html, "html"))

        with smtplib.SMTP(smtp_server, smtp_port) as server:
            server.login(sender_email, app_password)
            server.send_message(message)

        logger.info("Email with DataFrame sent successfully!")

    except smtplib.SMTPAuthenticationError:
        logger.exception("SMTP authentication failed. Check email or app password")
