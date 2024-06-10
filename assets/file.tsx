import { DialogClose } from "@radix-ui/react-dialog";
import type { ActionFunction } from "@remix-run/node";
import { useNavigate } from "@remix-run/react";
import { Trans } from "react-i18next";
import { $path } from "remix-routes";
import { useGlobalSubmittingState } from "remix-utils/use-global-navigation-state";
import { z } from "zod";

import { resetPassword } from "~/api";

import { Button } from "~/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "~/components/ui/dialog";

import { Email, Form } from "~/features/form";
import { ApplicationIcon } from "~/features/global/application-icons";
import i18next from "~/features/i18n/i18n.server";
import { redirectWithToast } from "~/features/toast/toast.server";

import { getValidatedFormDataWithResponse } from "~/utils/get-validated-form-data-with-response.server";
import { handleRemoteQuery } from "~/utils/handle-remote-query.server";

const schema = z.object({
	email: z.string().email(),
});
const ns = "reset-password" satisfies Ns;
export const action: ActionFunction = async ({ request }) => {
	const locale = await i18next.getLocale(request);
	const t = await i18next.getFixedT(locale, ns);
	const title = t("toast.title", "Reset password");
	const tt = t("toast.title", { defaultValue: "Reset password", namespace: "ns" });
	const { response, data } = await getValidatedFormDataWithResponse(
		request,
		schema,
		{
			title,
			text: t("toast.validation.error", "There is an error in the form"),
			iconType: "password",
			variant: "destructive",
		},
	);
	if (response) return response;
	return await handleRemoteQuery(
		async () => {
			await resetPassword(data);
			return redirectWithToast($path("/login"), {
				title,
				text: t(
					"toast.text.success",
					"An email has been sent",
				),
				iconType: "password",
				variant: "default",
			});
		},
		{
			title,
			text: t(
				"toast.text.error",
				"An error has occurred while trying to reset the password.",
			),
			variant: "destructive",
			iconType: "password",
		},
	);
};

export default function ResetPasswordDialog() {
	const navigate = useNavigate();
	const state = useGlobalSubmittingState();

	return (
		<Dialog
			defaultOpen
			onOpenChange={(isOpen) => {
				if (!isOpen) {
					navigate($path("/login"));
				}
			}}
		>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						<Trans ns={ns} i18nKey="dialog.title">
							Reset password
						</Trans>
					</DialogTitle>
					<DialogDescription>
						<Trans ns={ns} i18nKey="dialog.description">
							Enter your email address in the form below and we will send
							<br />
							you further instructions on how to reset your password.
						</Trans>
					</DialogDescription>
				</DialogHeader>

				<Form method="POST" schema={schema} defaultValue={{ email: "" }}>
					<Email />
					<DialogFooter>
						<DialogClose asChild>
							<Button
								variant="outline"
								isLoading={state !== "idle"}
								icon={<ApplicationIcon icon="close" />}
								type="reset"
							>
								<Trans ns={ns} i18nKey="button.clear">
									Clear
								</Trans>
							</Button>
						</DialogClose>
						<Button
							type="submit"
							isLoading={state !== "idle"}
							icon={<ApplicationIcon icon="send" />}
						>
							<Trans ns={ns} i18nKey="button.submit">
								Reset password
							</Trans>
						</Button>
					</DialogFooter>
				</Form>
			</DialogContent>
		</Dialog>
	);
}
