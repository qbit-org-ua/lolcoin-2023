import React, { useState, useEffect, useRef } from "react";
import { classNames } from "primereact/utils";
import { FilterMatchMode, FilterOperator } from "primereact/api";
import { DataTable } from "primereact/datatable";
import { Column } from "primereact/column";
import { Toast } from "primereact/toast";
import { Button } from "primereact/button";
import { Rating } from "primereact/rating";
import { InputTextarea } from "primereact/inputtextarea";
import { RadioButton } from "primereact/radiobutton";
import { InputNumber } from "primereact/inputnumber";
import { Dialog } from "primereact/dialog";
import { InputText } from "primereact/inputtext";
import { MultiSelect } from "primereact/multiselect";

import useSwr from "swr";

const fetcher = (url) => fetch(url, {cache: "no-store"}).then((res) => res.json());

export default function Home() {
  let emptyTransfer = {
    receiver: {},
    transferAmount: 0,
    senderSeedPhrase: "",
  };

  const {
    data: usersData,
    error: usersError,
    mutate: usersMutate,
  } = useSwr("/data.json", fetcher, { refreshInterval: 5000 });
  const [users, setUsers] = useState(null);
  const [transferDialog, setTransferDialog] = useState(false);
  const [transfer, setTransfer] = useState(emptyTransfer);
  const [selectedUsers, setSelectedUsers] = useState(null);
  const [submitted, setSubmitted] = useState(false);
  const [filters, setFilters] = useState(null);
  const toast = useRef(null);
  const dt = useRef(null);

  useEffect(() => {
    setFilters({
      fullName: { value: null, matchMode: FilterMatchMode.CONTAINS },
      schoolGrade: { value: null, matchMode: FilterMatchMode.CONTAINS },
    });

    if (!usersData) {
      return;
    }
    setUsers(
      usersData.map((user) => {
        return {
          ...user,
          balance: parseInt(user.balance),
        };
      })
    );
  }, [usersData, setUsers, usersError]);

  const formatCurrency = (value) => {
    return `${(value / 100).toLocaleString("uk-UK")} ЛОЛ`;
  };

  const hideDialog = () => {
    setSubmitted(false);
    setTransferDialog(false);
  };

  const sendTransfer = async () => {
    setSubmitted(true);
    if (!transfer.senderSeedPhrase || !transfer.transferAmount) {
      console.log(
        "hmm",
        transfer,
        !transfer.senderSeedPhrase,
        !transfer.transferAmount
      );
      return;
    }
    toast.current.show({
      severity: "success",
      summary: "Відпралвяємо коїни...",
      detail: "ЛОЛкоїни ще в дорозі",
      life: 10000,
    });

    const receiverFullName = transfer.receiver.fullName;
    const transferAmount = Math.round(transfer.transferAmount * 100) / 100;
    // TODO: sign transaction and don't send transfer object, so we don't need to leak the seed phrase!!!
    const request = {
      transfer_amount: Math.round(transfer.transferAmount * 100).toString(),
      sender_seed_phrase: transfer.senderSeedPhrase,
      receiver_account_id: transfer.receiver.accountId,
      //transaction: base64 blob
    };

    try {
      //const response = await fetch("https://coins.summerschool.lol/send-transfer", {
      const response = await fetch("/send-transfer", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(request),
      }).then((response) => response.json());
      if (response.status === "ok") {
        toast.current.show({
          severity: "success",
          summary: "Це успіх!",
          detail: (
            <>
              <p>{`${transferAmount} ЛОЛкоїн(ів) відправлено до користувача ${receiverFullName}!`}</p>
              <p>
                Баланси будут оновлені автоматично за декілька секунд,
                зачекайте...
              </p>
              <p>
                <a
                  target="_blank"
                  rel="noreferrer"
                  href={`https://explorer.mainnet.near.org/transactions/${response.transaction_hash}`}
                >
                  Подивитись транзакцію на NEAR Explorer
                </a>
              </p>
            </>
          ),
          life: 10000,
        });
      } else {
        toast.current.show({
          severity: "error",
          summary: "От халепа!",
          detail: response.error_message,
          life: 5000,
        });
      }
    } catch (error) {
      toast.current.show({
        severity: "error",
        summary: "От халепа!",
        detail: "Щось сталося",
        life: 5000,
      });
      console.log("SEND ERROR:", error);
    }

    setTransferDialog(false);
    setTransfer(emptyTransfer);
  };

  const transferTokens = (user) => {
    setTransfer({ ...emptyTransfer, receiver: { ...user } });
    setTransferDialog(true);
  };

  const exportCSV = () => {
    dt.current.exportCSV();
  };

  const onInputChange = (e, name) => {
    const val = (e.target && e.target.value) || "";
    let _transfer = { ...transfer };
    console.log("v", val, _transfer);
    _transfer[`${name}`] = val;

    setTransfer(_transfer);
  };

  const onInputNumberChange = (e, name) => {
    const val = e.value || 0;
    let _transfer = { ...transfer };
    _transfer[`${name}`] = val;

    setTransfer(_transfer);
  };

  const priceBodyTemplate = (rowData) => {
    return formatCurrency(rowData.balance);
  };

  const actionBodyTemplate = (rowData) => {
    return (
      <React.Fragment>
        <Button
          icon="pi pi-send"
          className="p-button-rounded p-button-success mr-2"
          onClick={() => transferTokens(rowData)}
        />
      </React.Fragment>
    );
  };

  const schoolGradeFilterTemplate = (options) => {
    return (
      <MultiSelect
        display="chip"
        value={options.value}
        options={[
          { label: "5 клас", value: "5" },
          { label: "6 клас", value: "6" },
          { label: "7 клас", value: "7" },
          { label: "8 клас", value: "8" },
          { label: "9 клас", value: "9" },
          { label: "10 клас", value: "10" },
        ]}
        onChange={(e) => {
          console.log("MULTI", options, e.value, filters);
          return options.filterCallback(e.value, options.index);
        }}
        placeholder="Будь-який"
      />
    );
  };

  const header = (
    <div className="flex flex-column md:flex-row md:align-items-center justify-content-between">
      <div className="mt-3 md:mt-0 flex justify-content-end">
        <Button
          icon="pi pi-upload"
          className="p-button-help p-button-rounded"
          onClick={exportCSV}
          tooltip="Export"
          tooltipOptions={{ position: "bottom" }}
        />
      </div>
    </div>
  );
  const transferDialogFooter = (
    <React.Fragment>
      <Button
        label="Охрана, отмєна"
        icon="pi pi-times"
        className="p-button-text"
        onClick={hideDialog}
      />
      <Button label="Відправити" icon="pi pi-check" onClick={sendTransfer} />
    </React.Fragment>
  );

  return (
    <div className="datatable-crud-demo surface-card p-4 border-round shadow-2">
      <Toast ref={toast} />

      <div className="text-3xl text-800 font-bold mb-4">ЛОЛкоїн</div>

      <DataTable
        value={users}
        dataKey="id"
        header={header}
        responsiveLayout="scroll"
        sortField="balance"
        sortOrder={-1}
        filters={filters}
        filterDisplay="row"
      >
        <Column
          field="fullName"
          header="Імʼя"
          sortable
          filter
          showFilterMatchModes={false}
          style={{ minWidth: "16rem" }}
        />
        <Column
          field="schoolGrade"
          header="Клас"
          sortable
          filter
          //filterElement={schoolGradeFilterTemplate}
          showFilterMatchModes={false}
          filterMenuStyle={{ width: "8rem" }}
          style={{ width: "14rem" }}
        />
        <Column
          field="balance"
          header="Баланс"
          body={priceBodyTemplate}
          sortable
          style={{ minWidth: "8rem" }}
        />
        <Column
          body={actionBodyTemplate}
          exportable={false}
          style={{ minWidth: "4rem" }}
        />
      </DataTable>

      <Dialog
        visible={transferDialog}
        breakpoints={{ "960px": "75vw", "640px": "100vw" }}
        style={{ width: "40vw" }}
        header="Відправка ЛОЛкоїнів"
        modal
        className="p-fluid"
        footer={transferDialogFooter}
        onHide={hideDialog}
      >
        <div className="field">
          <label htmlFor="receiver_name">Отримувач</label>
          <InputText
            id="receiver_name"
            value={transfer.receiver.fullName}
            disabled
          />
        </div>
        <div className="field">
          <label htmlFor="receiver_account_id">
            Обліковий запис отримувача в NEAR
          </label>
          <InputText
            id="receiver_account_id"
            value={transfer.receiver.accountId}
            disabled
          />
        </div>

        <div className="field">
          <label htmlFor="transfer_amount">ЛОЛ, скільки?</label>
          <InputNumber
            id="transfer_amount"
            value={transfer.transferAmount}
            onValueChange={(e) => onInputNumberChange(e, "transferAmount")}
            suffix=" ЛОЛ"
            locale="uk-UK"
            autoFocus
            required
          />
          {submitted && !transfer.transferAmount && (
            <small className="p-error">Сумма переводу є обовʼязковою.</small>
          )}
        </div>

        <div className="field">
          <label htmlFor="sender_seed_phrase">Кодова фраза відправника</label>
          <InputText
            id="sender_seed_phrase"
            value={transfer.senderSeedPhrase}
            onChange={(e) => onInputChange(e, "senderSeedPhrase")}
            required
            className={classNames({
              "p-invalid": submitted && !transfer.senderSeedPhrase,
            })}
          />
          {submitted && !transfer.senderSeedPhrase && (
            <small className="p-error">
              Кодова фраза відправника є обовʼязковою.
            </small>
          )}
        </div>
      </Dialog>
    </div>
  );
}
