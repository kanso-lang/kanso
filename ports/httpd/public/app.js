// A small client for the todo-backend API this server implements.
const list = document.getElementById("list");

async function refresh() {
  const todos = await (await fetch("/todos")).json();
  list.replaceChildren(...todos.map(item));
}

function item(todo) {
  const li = document.createElement("li");
  li.textContent = todo.title;
  li.className = todo.completed ? "done" : "";
  li.onclick = async () => {
    await fetch(todo.url, {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ completed: !todo.completed }),
    });
    refresh();
  };
  return li;
}

document.getElementById("add").onsubmit = async (event) => {
  event.preventDefault();
  const title = document.getElementById("title");
  await fetch("/todos", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ title: title.value }),
  });
  title.value = "";
  refresh();
};

refresh();
