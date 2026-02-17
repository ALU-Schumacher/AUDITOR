import os
import smtplib
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from logging import Logger

import pandas as pd
from dotenv import load_dotenv


async def send_email(
    smtp_server: str, smtp_port: int, df_sum: pd.DataFrame, logger: Logger
) -> None:
    try:
        load_dotenv()
        sender_email = os.getenv("SENDER_EMAIL")
        app_password = os.getenv("PASSWORD")
        receiver_email = os.getenv("RECEIVER_EMAIL")

        if not sender_email or not app_password or not receiver_email:
            raise ValueError("Missing required email environment variables")

        html_table = df_sum.to_html(index=False, border=0, justify="center")

        subject = "Weekly Utilisation Report"
        body_html = f"""
        <html>
          <body>
            <h2>Weekly Utilisation Summary</h2>
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
