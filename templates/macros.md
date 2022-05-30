{%- macro toc(contents) %}
{%- for item in contents -%}
* {{ item.date }} - [{{ item.title }}]({{ item.target }})
{% endfor %}
{% endmacro -%}
